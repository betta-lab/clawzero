use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::agent::event::AgentEvent;
use crate::agent::factory::AgentFactory;
use crate::config::types::SlackConfig;
use crate::error::ClawError;
use crate::gateway::event_handler::BotEventHandler;
use crate::gateway::session_map::{SessionMap, ThreadKey};
use crate::gateway::slack::api::SlackApi;
use crate::gateway::slack::socket::{SlackEvent, SlackSocket};
use crate::session::store::SessionStore;

/// Run the Slack gateway — connects via Socket Mode and responds to messages.
pub async fn run_slack_gateway(
    factory: Arc<AgentFactory>,
    session_store: SessionStore,
    session_map: Arc<SessionMap>,
    config: &SlackConfig,
) -> Result<(), ClawError> {
    let app_token = config
        .resolve_app_token()
        .ok_or_else(|| ClawError::Gateway("Slack app_token not configured".into()))?;
    let bot_token = config
        .resolve_bot_token()
        .ok_or_else(|| ClawError::Gateway("Slack bot_token not configured".into()))?;

    let api = Arc::new(SlackApi::new(&bot_token));

    // Verify authentication
    let bot_user_id = api.auth_test().await?;
    tracing::info!("Slack bot authenticated as {bot_user_id}");

    let mut socket = SlackSocket::connect(&app_token).await?;
    let session_store = Arc::new(session_store);

    // Active threads: thread_key_string → mpsc sender
    let mut threads: HashMap<String, mpsc::Sender<String>> = HashMap::new();

    loop {
        let event = match socket.next_event().await {
            Ok(Some(event)) => event,
            Ok(None) => {
                tracing::info!("Slack WebSocket closed, reconnecting...");
                socket.reconnect().await?;
                continue;
            }
            Err(e) => {
                tracing::error!("Slack WebSocket error: {e}, reconnecting...");
                tokio::time::sleep(Duration::from_secs(5)).await;
                socket.reconnect().await?;
                continue;
            }
        };

        let (envelope_id, slack_event) = event;

        // Acknowledge immediately
        socket.acknowledge(&envelope_id).await?;

        let (channel, text, thread_ts) = match slack_event {
            SlackEvent::AppMention {
                channel,
                text,
                ts,
                thread_ts,
                ..
            } => {
                // Strip the @mention prefix
                let clean_text = strip_mention(&text, &bot_user_id);
                let effective_thread_ts = thread_ts.unwrap_or(ts);
                (channel, clean_text, effective_thread_ts)
            }
            SlackEvent::Message {
                channel,
                text,
                ts,
                thread_ts,
                ..
            } => {
                // Only respond to DMs (channel starts with D) or thread replies
                if !channel.starts_with('D') && thread_ts.is_none() {
                    continue;
                }
                let effective_thread_ts = thread_ts.unwrap_or(ts);
                (channel, text, effective_thread_ts)
            }
            SlackEvent::Disconnect { reason } => {
                tracing::info!("Slack disconnect requested: {reason}");
                socket.reconnect().await?;
                continue;
            }
        };

        if text.trim().is_empty() {
            continue;
        }

        let thread_key_str = format!("{channel}:{thread_ts}");

        // Try to send to existing thread task
        if let Some(tx) = threads.get(&thread_key_str) {
            if tx.try_send(text.clone()).is_ok() {
                continue;
            }
            // Channel full or closed — will respawn below
            threads.remove(&thread_key_str);
        }

        // Spawn new thread task
        let (tx, rx) = mpsc::channel::<String>(32);
        let _ = tx.try_send(text);
        threads.insert(thread_key_str.clone(), tx);

        let thread_key = ThreadKey {
            platform: "slack".into(),
            thread_id: thread_key_str,
        };

        let factory = Arc::clone(&factory);
        let session_store = Arc::clone(&session_store);
        let session_map = Arc::clone(&session_map);
        let api = Arc::clone(&api);
        let channel = channel.clone();
        let thread_ts = thread_ts.clone();

        tokio::spawn(async move {
            if let Err(e) = run_slack_thread(
                factory,
                session_store,
                session_map,
                api,
                rx,
                thread_key,
                channel,
                thread_ts,
            )
            .await
            {
                tracing::error!("Slack thread error: {e}");
            }
        });
    }
}

/// Handle messages for a single Slack thread.
async fn run_slack_thread(
    factory: Arc<AgentFactory>,
    session_store: Arc<SessionStore>,
    session_map: Arc<SessionMap>,
    api: Arc<SlackApi>,
    mut rx: mpsc::Receiver<String>,
    thread_key: ThreadKey,
    channel: String,
    thread_ts: String,
) -> Result<(), ClawError> {
    // Check if we have an existing session for this thread
    let existing_session = session_map.get(&thread_key);

    let mut agent = if let Some(ref session_id) = existing_session {
        match session_store.resume_session(session_id) {
            Ok((writer, messages)) => {
                tracing::info!("Resuming session {session_id} for thread");
                factory.create_resumed(writer, messages)
            }
            Err(e) => {
                tracing::warn!("Failed to resume session {session_id}: {e}, creating new");
                create_new_session(&factory, &session_store, &session_map, &thread_key)?
            }
        }
    } else {
        create_new_session(&factory, &session_store, &session_map, &thread_key)?
    };

    while let Some(text) = rx.recv().await {
        // Post a placeholder message
        let msg_ts = api
            .post_message(&channel, "...", Some(&thread_ts))
            .await?;

        // Channel for streaming events
        let (event_tx, mut event_rx) = mpsc::channel::<AgentEvent>(256);

        // Spawn the agent run
        let agent_handle = tokio::spawn({
            async move {
                agent
                    .run(text, |event| {
                        let _ = event_tx.try_send(event.clone());
                    })
                    .await;
                agent
            }
        });

        // Process events and update Slack message
        let api_clone = Arc::clone(&api);
        let channel_clone = channel.clone();
        let msg_ts_clone = msg_ts.clone();

        let update_handle = tokio::spawn(async move {
            let mut handler = BotEventHandler::new(Duration::from_millis(500));
            while let Some(event) = event_rx.recv().await {
                if let Some(text) = handler.handle_event(&event) {
                    if !text.is_empty() {
                        let _ = api_clone
                            .update_message(&channel_clone, &msg_ts_clone, &text)
                            .await;
                    }
                }
            }
            // Final update
            let final_text = handler.finalize();
            if !final_text.is_empty() {
                let _ = api_clone
                    .update_message(&channel_clone, &msg_ts_clone, &final_text)
                    .await;
            }
        });

        // Wait for agent to finish
        agent = agent_handle
            .await
            .map_err(|e| ClawError::Gateway(format!("Agent task panicked: {e}")))?;

        // Wait for update handler to finish
        let _ = update_handle.await;
    }

    Ok(())
}

fn create_new_session(
    factory: &AgentFactory,
    session_store: &SessionStore,
    session_map: &SessionMap,
    thread_key: &ThreadKey,
) -> Result<crate::agent::r#loop::Agent, ClawError> {
    let writer = session_store.create_session(factory.model())?;
    let session_id = writer.session_id().to_string();
    session_map.put(thread_key, session_id)?;
    Ok(factory.create_with_session(writer))
}

/// Strip @mention prefix from message text.
fn strip_mention(text: &str, bot_user_id: &str) -> String {
    let mention = format!("<@{bot_user_id}>");
    text.replace(&mention, "").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_mention_removes_bot_mention() {
        assert_eq!(strip_mention("<@U123> hello", "U123"), "hello");
    }

    #[test]
    fn strip_mention_no_mention() {
        assert_eq!(strip_mention("hello world", "U123"), "hello world");
    }

    #[test]
    fn strip_mention_multiple() {
        assert_eq!(
            strip_mention("<@U123> hey <@U123>", "U123"),
            "hey"
        );
    }
}

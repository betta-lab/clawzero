use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serenity::all::{
    Context, CreateMessage, EditMessage, EventHandler, GatewayIntents, Message as DiscordMessage,
    Ready, UserId,
};
use serenity::async_trait;
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::agent::event::AgentEvent;
use crate::agent::factory::AgentFactory;
use crate::config::types::DiscordConfig;
use crate::error::ClawError;
use crate::gateway::event_handler::BotEventHandler;
use crate::gateway::session_map::{SessionMap, ThreadKey};
use crate::session::store::SessionStore;

struct DiscordHandler {
    factory: Arc<AgentFactory>,
    session_store: Arc<SessionStore>,
    session_map: Arc<SessionMap>,
    threads: Arc<Mutex<HashMap<String, mpsc::Sender<(String, DiscordMessage)>>>>,
    bot_user_id: Arc<RwLock<Option<UserId>>>,
}

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        let mut bot_id = self.bot_user_id.write().await;
        *bot_id = Some(ready.user.id);
        tracing::info!("Discord bot ready as {}", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: DiscordMessage) {
        // Ignore bot messages
        if msg.author.bot {
            return;
        }

        let bot_id = {
            let id = self.bot_user_id.read().await;
            match *id {
                Some(id) => id,
                None => return,
            }
        };

        // Check if we should respond: DM or @mention
        let is_dm = msg.guild_id.is_none();
        let is_mentioned = msg.mentions.iter().any(|u| u.id == bot_id);

        if !is_dm && !is_mentioned {
            return;
        }

        let text = extract_text_without_mention(&msg.content, bot_id.get());
        if text.is_empty() {
            return;
        }

        // Determine thread key
        let thread_id = if let Some(thread) = msg.thread.as_ref() {
            thread.id.to_string()
        } else {
            msg.channel_id.to_string()
        };
        let thread_key_str = format!("{}:{}", msg.channel_id, thread_id);

        // Try sending to existing thread task
        let mut threads = self.threads.lock().await;
        if let Some(tx) = threads.get(&thread_key_str) {
            if tx.try_send((text.clone(), msg.clone())).is_ok() {
                return;
            }
            threads.remove(&thread_key_str);
        }

        // Spawn new thread task
        let (tx, rx) = mpsc::channel(32);
        let _ = tx.try_send((text, msg));
        threads.insert(thread_key_str.clone(), tx);
        drop(threads);

        let thread_key = ThreadKey {
            platform: "discord".into(),
            thread_id: thread_key_str,
        };

        let factory = Arc::clone(&self.factory);
        let session_store = Arc::clone(&self.session_store);
        let session_map = Arc::clone(&self.session_map);
        let http = ctx.http.clone();

        tokio::spawn(async move {
            if let Err(e) = run_discord_thread(
                factory,
                session_store,
                session_map,
                http,
                rx,
                thread_key,
            )
            .await
            {
                tracing::error!("Discord thread error: {e}");
            }
        });
    }
}

async fn run_discord_thread(
    factory: Arc<AgentFactory>,
    session_store: Arc<SessionStore>,
    session_map: Arc<SessionMap>,
    http: Arc<serenity::http::Http>,
    mut rx: mpsc::Receiver<(String, DiscordMessage)>,
    thread_key: ThreadKey,
) -> Result<(), ClawError> {
    // Check for existing session
    let existing_session = session_map.get(&thread_key);

    let mut agent = if let Some(ref session_id) = existing_session {
        match session_store.resume_session(session_id) {
            Ok((writer, messages)) => {
                tracing::info!("Resuming Discord session {session_id}");
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

    while let Some((text, msg)) = rx.recv().await {
        // Post placeholder
        let placeholder = msg
            .channel_id
            .send_message(&http, CreateMessage::new().content("..."))
            .await
            .map_err(|e| ClawError::Gateway(format!("Discord send failed: {e}")))?;

        let (event_tx, mut event_rx) = mpsc::channel::<AgentEvent>(256);

        let agent_handle = tokio::spawn(async move {
            agent
                .run(text, |event| {
                    let _ = event_tx.try_send(event.clone());
                })
                .await;
            agent
        });

        let http_clone = Arc::clone(&http);
        let channel_id = msg.channel_id;
        let placeholder_id = placeholder.id;

        let update_handle = tokio::spawn(async move {
            let mut handler = BotEventHandler::new(Duration::from_millis(500));
            while let Some(event) = event_rx.recv().await {
                if let Some(text) = handler.handle_event(&event) {
                    if !text.is_empty() {
                        // Discord max message length is 2000 chars
                        let truncated = if text.len() > 1990 {
                            format!("{}...", &text[..1990])
                        } else {
                            text
                        };
                        let _ = channel_id
                            .edit_message(&http_clone, placeholder_id, EditMessage::new().content(&truncated))
                            .await;
                    }
                }
            }
            let final_text = handler.finalize();
            if !final_text.is_empty() {
                let truncated = if final_text.len() > 1990 {
                    format!("{}...", &final_text[..1990])
                } else {
                    final_text
                };
                let _ = channel_id
                    .edit_message(&http_clone, placeholder_id, EditMessage::new().content(&truncated))
                    .await;
            }
        });

        agent = agent_handle
            .await
            .map_err(|e| ClawError::Gateway(format!("Agent task panicked: {e}")))?;

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

/// Extract message text, removing the bot @mention.
pub fn extract_text_without_mention(content: &str, bot_user_id: u64) -> String {
    let mention = format!("<@{bot_user_id}>");
    content.replace(&mention, "").trim().to_string()
}

/// Run the Discord gateway.
pub async fn run_discord_gateway(
    factory: Arc<AgentFactory>,
    session_store: SessionStore,
    session_map: Arc<SessionMap>,
    config: &DiscordConfig,
) -> Result<(), ClawError> {
    let bot_token = config
        .resolve_bot_token()
        .ok_or_else(|| ClawError::Gateway("Discord bot_token not configured".into()))?;

    let handler = DiscordHandler {
        factory,
        session_store: Arc::new(session_store),
        session_map,
        threads: Arc::new(Mutex::new(HashMap::new())),
        bot_user_id: Arc::new(RwLock::new(None)),
    };

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = serenity::Client::builder(&bot_token, intents)
        .event_handler(handler)
        .await
        .map_err(|e| ClawError::Gateway(format!("Discord client build failed: {e}")))?;

    tracing::info!("Starting Discord gateway...");
    client
        .start()
        .await
        .map_err(|e| ClawError::Gateway(format!("Discord client error: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_text_without_mention_basic() {
        assert_eq!(
            extract_text_without_mention("<@123456> hello", 123456),
            "hello"
        );
    }

    #[test]
    fn extract_text_without_mention_no_mention() {
        assert_eq!(
            extract_text_without_mention("hello world", 123456),
            "hello world"
        );
    }

    #[test]
    fn discord_determines_thread_key() {
        let key = ThreadKey {
            platform: "discord".into(),
            thread_id: "123:456".into(),
        };
        assert_eq!(key.platform, "discord");
        assert_eq!(key.thread_id, "123:456");
    }
}

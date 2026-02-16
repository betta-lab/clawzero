use std::sync::Arc;

use axum::Router;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use tokio::sync::mpsc;

use crate::agent::event::AgentEvent;
use crate::agent::factory::AgentFactory;
use crate::config::types::WebuiConfig;
use crate::error::ClawError;
use crate::gateway::session_map::{SessionMap, ThreadKey};
use crate::gateway::webui::messages::{ClientMessage, ServerEvent};
use crate::session::store::SessionStore;

struct AppState {
    factory: Arc<AgentFactory>,
    session_store: Arc<SessionStore>,
    session_map: Arc<SessionMap>,
}

/// Run the WebUI gateway — starts an HTTP + WebSocket server.
pub async fn run_webui_gateway(
    factory: Arc<AgentFactory>,
    session_store: SessionStore,
    session_map: Arc<SessionMap>,
    config: &WebuiConfig,
) -> Result<(), ClawError> {
    let state = Arc::new(AppState {
        factory,
        session_store: Arc::new(session_store),
        session_map,
    });

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/ws", get(websocket_handler))
        .route("/api/sessions", get(sessions_handler))
        .with_state(state);

    let addr = format!("{}:{}", config.host(), config.port());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| ClawError::Gateway(format!("Failed to bind {addr}: {e}")))?;

    tracing::info!("WebUI gateway listening on http://{addr}");

    axum::serve(listener, app)
        .await
        .map_err(|e| ClawError::Gateway(format!("WebUI server error: {e}")))?;

    Ok(())
}

async fn index_handler() -> Html<&'static str> {
    Html(include_str!("ui.html"))
}

async fn sessions_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.session_store.list_sessions() {
        Ok(sessions) => {
            let list: Vec<serde_json::Value> = sessions
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "session_id": s.session_id,
                        "model": s.model,
                        "created_at": s.created_at,
                        "message_count": s.message_count,
                    })
                })
                .collect();
            axum::Json(serde_json::json!({ "sessions": list })).into_response()
        }
        Err(e) => {
            let body = serde_json::json!({ "error": e.to_string() });
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(body),
            )
                .into_response()
        }
    }
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_connection(socket, state))
}

async fn handle_connection(socket: WebSocket, state: Arc<AppState>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    use futures_util::{SinkExt, StreamExt};

    // Generate a unique connection ID for session mapping
    let conn_id = ulid::Ulid::new().to_string().to_lowercase();
    let thread_key = ThreadKey {
        platform: "webui".into(),
        thread_id: conn_id,
    };

    let mut agent = None;
    let mut current_session_id: Option<String> = None;

    while let Some(msg) = ws_receiver.next().await {
        let msg = match msg {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) => break,
            Err(e) => {
                tracing::warn!("WebSocket receive error: {e}");
                break;
            }
            _ => continue,
        };

        let client_msg: ClientMessage = match serde_json::from_str(&msg) {
            Ok(m) => m,
            Err(e) => {
                let err = ServerEvent::Error {
                    message: format!("Invalid message: {e}"),
                };
                let _ = ws_sender
                    .send(Message::Text(serde_json::to_string(&err).unwrap().into()))
                    .await;
                continue;
            }
        };

        match client_msg {
            ClientMessage::NewSession => {
                // Create a new session
                match create_new_session(
                    &state.factory,
                    &state.session_store,
                    &state.session_map,
                    &thread_key,
                ) {
                    Ok(new_agent) => {
                        let sid = new_agent.session_id().unwrap_or_default().to_string();
                        current_session_id = Some(sid.clone());
                        agent = Some(new_agent);
                        let event = ServerEvent::SessionCreated { session_id: sid };
                        let _ = ws_sender
                            .send(Message::Text(serde_json::to_string(&event).unwrap().into()))
                            .await;
                    }
                    Err(e) => {
                        let event = ServerEvent::Error {
                            message: format!("Failed to create session: {e}"),
                        };
                        let _ = ws_sender
                            .send(Message::Text(serde_json::to_string(&event).unwrap().into()))
                            .await;
                    }
                }
            }
            ClientMessage::ResumeSession { session_id } => {
                match state.session_store.resume_session(&session_id) {
                    Ok((writer, messages)) => {
                        let resumed = state.factory.create_resumed(writer, messages);
                        current_session_id = Some(session_id.clone());
                        agent = Some(resumed);
                        let event = ServerEvent::SessionCreated { session_id };
                        let _ = ws_sender
                            .send(Message::Text(serde_json::to_string(&event).unwrap().into()))
                            .await;
                    }
                    Err(e) => {
                        let event = ServerEvent::Error {
                            message: format!("Failed to resume session: {e}"),
                        };
                        let _ = ws_sender
                            .send(Message::Text(serde_json::to_string(&event).unwrap().into()))
                            .await;
                    }
                }
            }
            ClientMessage::Message { text } => {
                if text.trim().is_empty() {
                    continue;
                }

                // Auto-create session if none exists
                if agent.is_none() {
                    match create_new_session(
                        &state.factory,
                        &state.session_store,
                        &state.session_map,
                        &thread_key,
                    ) {
                        Ok(new_agent) => {
                            let sid = new_agent.session_id().unwrap_or_default().to_string();
                            current_session_id = Some(sid.clone());
                            agent = Some(new_agent);
                            let event = ServerEvent::SessionCreated { session_id: sid };
                            let _ = ws_sender
                                .send(Message::Text(serde_json::to_string(&event).unwrap().into()))
                                .await;
                        }
                        Err(e) => {
                            let event = ServerEvent::Error {
                                message: format!("Failed to create session: {e}"),
                            };
                            let _ = ws_sender
                                .send(Message::Text(serde_json::to_string(&event).unwrap().into()))
                                .await;
                            continue;
                        }
                    }
                }

                // Take ownership of agent for the spawn
                let mut owned_agent = agent.take().unwrap();
                let (event_tx, mut event_rx) = mpsc::channel::<AgentEvent>(256);

                let agent_handle = tokio::spawn(async move {
                    owned_agent
                        .run(text, |event| {
                            let _ = event_tx.try_send(event.clone());
                        })
                        .await;
                    owned_agent
                });

                // Stream events to WebSocket
                while let Some(event) = event_rx.recv().await {
                    let server_event: Option<ServerEvent> = (&event).into();
                    if let Some(se) = server_event {
                        let json = serde_json::to_string(&se).unwrap();
                        if ws_sender.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                }

                // Recover the agent
                match agent_handle.await {
                    Ok(recovered) => {
                        agent = Some(recovered);
                    }
                    Err(e) => {
                        tracing::error!("Agent task panicked: {e}");
                        let event = ServerEvent::Error {
                            message: format!("Agent task panicked: {e}"),
                        };
                        let _ = ws_sender
                            .send(Message::Text(serde_json::to_string(&event).unwrap().into()))
                            .await;
                    }
                }
            }
        }
    }

    tracing::debug!(
        "WebSocket connection closed, session: {:?}",
        current_session_id
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_returns_html() {
        // Verify include_str! compiles (ui.html must exist)
        let html = include_str!("ui.html");
        assert!(html.contains("<!DOCTYPE html>"));
    }
}

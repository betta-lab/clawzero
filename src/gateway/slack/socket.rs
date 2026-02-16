use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::error::ClawError;

/// Events received from Slack Socket Mode.
#[derive(Debug, Clone)]
pub enum SlackEvent {
    AppMention {
        channel: String,
        thread_ts: Option<String>,
        text: String,
        user: String,
        ts: String,
    },
    Message {
        channel: String,
        text: String,
        user: String,
        ts: String,
        thread_ts: Option<String>,
    },
    Disconnect {
        reason: String,
    },
}

/// Slack Socket Mode WebSocket connection.
pub struct SlackSocket {
    ws_stream: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    app_token: String,
    client: reqwest::Client,
}

impl SlackSocket {
    /// Connect to Slack Socket Mode.
    /// 1. POST apps.connections.open with xapp-token → get wss:// URL
    /// 2. Connect via WebSocket
    pub async fn connect(app_token: &str) -> Result<Self, ClawError> {
        let client = reqwest::Client::new();
        let resp = client
            .post("https://slack.com/api/apps.connections.open")
            .bearer_auth(app_token)
            .send()
            .await
            .map_err(|e| ClawError::WebSocket(format!("Failed to open connection: {e}")))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ClawError::WebSocket(format!("Failed to parse connection response: {e}")))?;

        if !body["ok"].as_bool().unwrap_or(false) {
            let err = body["error"].as_str().unwrap_or("unknown");
            return Err(ClawError::WebSocket(format!(
                "apps.connections.open failed: {err}"
            )));
        }

        let ws_url = body["url"]
            .as_str()
            .ok_or_else(|| ClawError::WebSocket("No URL in connection response".into()))?;

        let (ws_stream, _) = tokio_tungstenite::connect_async(ws_url)
            .await
            .map_err(|e| ClawError::WebSocket(format!("WebSocket connect failed: {e}")))?;

        tracing::info!("Connected to Slack Socket Mode");

        Ok(Self {
            ws_stream,
            app_token: app_token.to_string(),
            client,
        })
    }

    /// Read the next event from the WebSocket.
    /// Returns (envelope_id, event). Envelope ID must be acknowledged within 3 seconds.
    pub async fn next_event(&mut self) -> Result<Option<(String, SlackEvent)>, ClawError> {
        loop {
            let msg = match self.ws_stream.next().await {
                Some(Ok(msg)) => msg,
                Some(Err(e)) => {
                    return Err(ClawError::WebSocket(format!("WebSocket read error: {e}")));
                }
                None => return Ok(None),
            };

            let text = match msg {
                WsMessage::Text(t) => t.to_string(),
                WsMessage::Ping(_) => continue,
                WsMessage::Pong(_) => continue,
                WsMessage::Close(_) => return Ok(None),
                _ => continue,
            };

            let envelope: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| ClawError::WebSocket(format!("Invalid JSON from Slack: {e}")))?;

            let envelope_id = match envelope["envelope_id"].as_str() {
                Some(id) => id.to_string(),
                None => continue, // Not an envelope, skip (e.g. hello message)
            };

            // Check for disconnect request
            if envelope["type"].as_str() == Some("disconnect") {
                let reason = envelope["reason"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                return Ok(Some((envelope_id, SlackEvent::Disconnect { reason })));
            }

            // Parse event payload
            if let Some(event) = parse_event_payload(&envelope) {
                return Ok(Some((envelope_id, event)));
            }

            // Unknown event type — acknowledge but skip
            self.acknowledge(&envelope_id).await?;
        }
    }

    /// Acknowledge receipt of an envelope (must be done within 3 seconds).
    pub async fn acknowledge(&mut self, envelope_id: &str) -> Result<(), ClawError> {
        let ack = serde_json::json!({ "envelope_id": envelope_id });
        self.ws_stream
            .send(WsMessage::Text(ack.to_string().into()))
            .await
            .map_err(|e| ClawError::WebSocket(format!("Failed to send ack: {e}")))?;
        Ok(())
    }

    /// Reconnect (get new WebSocket URL and connect).
    pub async fn reconnect(&mut self) -> Result<(), ClawError> {
        let resp = self
            .client
            .post("https://slack.com/api/apps.connections.open")
            .bearer_auth(&self.app_token)
            .send()
            .await
            .map_err(|e| ClawError::WebSocket(format!("Reconnect failed: {e}")))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ClawError::WebSocket(format!("Reconnect parse failed: {e}")))?;

        if !body["ok"].as_bool().unwrap_or(false) {
            let err = body["error"].as_str().unwrap_or("unknown");
            return Err(ClawError::WebSocket(format!(
                "Reconnect apps.connections.open failed: {err}"
            )));
        }

        let ws_url = body["url"]
            .as_str()
            .ok_or_else(|| ClawError::WebSocket("No URL in reconnect response".into()))?;

        let (ws_stream, _) = tokio_tungstenite::connect_async(ws_url)
            .await
            .map_err(|e| ClawError::WebSocket(format!("WebSocket reconnect failed: {e}")))?;

        self.ws_stream = ws_stream;
        tracing::info!("Reconnected to Slack Socket Mode");
        Ok(())
    }
}

/// Parse the event payload from a Socket Mode envelope.
fn parse_event_payload(envelope: &serde_json::Value) -> Option<SlackEvent> {
    let payload = &envelope["payload"];
    let event = &payload["event"];

    let event_type = event["type"].as_str()?;
    let channel = event["channel"].as_str()?.to_string();
    let text = event["text"].as_str().unwrap_or("").to_string();
    let user = event["user"].as_str().unwrap_or("").to_string();
    let ts = event["ts"].as_str().unwrap_or("").to_string();
    let thread_ts = event["thread_ts"].as_str().map(|s| s.to_string());

    match event_type {
        "app_mention" => Some(SlackEvent::AppMention {
            channel,
            thread_ts,
            text,
            user,
            ts,
        }),
        "message" => {
            // Skip bot messages and message_changed subtypes
            if event["bot_id"].is_string() || event["subtype"].is_string() {
                return None;
            }
            Some(SlackEvent::Message {
                channel,
                text,
                user,
                ts,
                thread_ts,
            })
        }
        _ => None,
    }
}

/// Format for acknowledge message.
pub fn acknowledge_json(envelope_id: &str) -> String {
    serde_json::json!({ "envelope_id": envelope_id }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_app_mention_event() {
        let envelope = serde_json::json!({
            "envelope_id": "env-123",
            "type": "events_api",
            "payload": {
                "event": {
                    "type": "app_mention",
                    "channel": "C12345",
                    "text": "<@U999> hello",
                    "user": "U123",
                    "ts": "1234567890.123456",
                    "thread_ts": "1234567890.000001"
                }
            }
        });
        let event = parse_event_payload(&envelope).unwrap();
        match event {
            SlackEvent::AppMention {
                channel,
                text,
                user,
                ts,
                thread_ts,
            } => {
                assert_eq!(channel, "C12345");
                assert_eq!(text, "<@U999> hello");
                assert_eq!(user, "U123");
                assert_eq!(ts, "1234567890.123456");
                assert_eq!(thread_ts, Some("1234567890.000001".to_string()));
            }
            _ => panic!("Expected AppMention"),
        }
    }

    #[test]
    fn parse_message_event() {
        let envelope = serde_json::json!({
            "envelope_id": "env-456",
            "type": "events_api",
            "payload": {
                "event": {
                    "type": "message",
                    "channel": "D999",
                    "text": "hello bot",
                    "user": "U456",
                    "ts": "1234567891.000001"
                }
            }
        });
        let event = parse_event_payload(&envelope).unwrap();
        match event {
            SlackEvent::Message {
                channel,
                text,
                user,
                ..
            } => {
                assert_eq!(channel, "D999");
                assert_eq!(text, "hello bot");
                assert_eq!(user, "U456");
            }
            _ => panic!("Expected Message"),
        }
    }

    #[test]
    fn parse_message_skips_bot() {
        let envelope = serde_json::json!({
            "envelope_id": "env-789",
            "type": "events_api",
            "payload": {
                "event": {
                    "type": "message",
                    "channel": "C123",
                    "text": "bot reply",
                    "user": "U123",
                    "ts": "1234567892.000001",
                    "bot_id": "B999"
                }
            }
        });
        assert!(parse_event_payload(&envelope).is_none());
    }

    #[test]
    fn parse_disconnect() {
        let envelope = serde_json::json!({
            "envelope_id": "env-disc",
            "type": "disconnect",
            "reason": "link_disabled"
        });

        // Disconnect is handled at the envelope level, not payload level
        assert_eq!(envelope["type"].as_str(), Some("disconnect"));
        let reason = envelope["reason"].as_str().unwrap_or("unknown");
        assert_eq!(reason, "link_disabled");
    }

    #[test]
    fn acknowledge_format() {
        let json = acknowledge_json("env-123");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["envelope_id"], "env-123");
    }
}

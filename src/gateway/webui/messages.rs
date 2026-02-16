use serde::{Deserialize, Serialize};

use crate::agent::event::AgentEvent;

/// Messages sent from the WebUI client to the server.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Message { text: String },
    NewSession,
    ResumeSession { session_id: String },
}

/// Events sent from the server to the WebUI client.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    SessionCreated {
        session_id: String,
    },
    TextDelta {
        text: String,
    },
    ToolCallStart {
        id: String,
        name: String,
    },
    ToolResult {
        id: String,
        name: String,
        input: serde_json::Value,
        output: String,
        is_error: bool,
    },
    Done {
        total_usage: UsageInfo,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Serialize)]
pub struct UsageInfo {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

impl From<&AgentEvent> for Option<ServerEvent> {
    fn from(event: &AgentEvent) -> Self {
        match event {
            AgentEvent::TextDelta(text) => Some(ServerEvent::TextDelta { text: text.clone() }),
            AgentEvent::ToolCallStart { id, name } => Some(ServerEvent::ToolCallStart {
                id: id.clone(),
                name: name.clone(),
            }),
            AgentEvent::ToolResult {
                id,
                name,
                input,
                output,
                is_error,
            } => Some(ServerEvent::ToolResult {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
                output: output.clone(),
                is_error: *is_error,
            }),
            AgentEvent::Done { total_usage } => Some(ServerEvent::Done {
                total_usage: UsageInfo {
                    input_tokens: total_usage.input_tokens,
                    output_tokens: total_usage.output_tokens,
                },
            }),
            AgentEvent::Error(msg) => Some(ServerEvent::Error {
                message: msg.clone(),
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::response::Usage;

    #[test]
    fn deserialize_message() {
        let json = r#"{"type":"message","text":"hello"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::Message { text } => assert_eq!(text, "hello"),
            _ => panic!("Expected Message"),
        }
    }

    #[test]
    fn deserialize_new_session() {
        let json = r#"{"type":"new_session"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::NewSession));
    }

    #[test]
    fn deserialize_resume_session() {
        let json = r#"{"type":"resume_session","session_id":"abc123"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::ResumeSession { session_id } => assert_eq!(session_id, "abc123"),
            _ => panic!("Expected ResumeSession"),
        }
    }

    #[test]
    fn serialize_session_created() {
        let event = ServerEvent::SessionCreated {
            session_id: "abc".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"session_created""#));
        assert!(json.contains(r#""session_id":"abc""#));
    }

    #[test]
    fn serialize_text_delta() {
        let event = ServerEvent::TextDelta {
            text: "hello".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"text_delta""#));
        assert!(json.contains(r#""text":"hello""#));
    }

    #[test]
    fn serialize_tool_call_start() {
        let event = ServerEvent::ToolCallStart {
            id: "t1".into(),
            name: "bash".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"tool_call_start""#));
    }

    #[test]
    fn serialize_tool_result() {
        let event = ServerEvent::ToolResult {
            id: "t1".into(),
            name: "bash".into(),
            input: serde_json::json!({"command": "ls"}),
            output: "file.txt".into(),
            is_error: false,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"tool_result""#));
        assert!(json.contains(r#""is_error":false"#));
    }

    #[test]
    fn serialize_done() {
        let event = ServerEvent::Done {
            total_usage: UsageInfo {
                input_tokens: 200,
                output_tokens: 150,
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"done""#));
        assert!(json.contains(r#""input_tokens":200"#));
    }

    #[test]
    fn serialize_error() {
        let event = ServerEvent::Error {
            message: "something failed".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"error""#));
    }

    #[test]
    fn convert_agent_text_delta() {
        let agent_event = AgentEvent::TextDelta("hi".into());
        let server_event: Option<ServerEvent> = (&agent_event).into();
        assert!(matches!(server_event, Some(ServerEvent::TextDelta { .. })));
    }

    #[test]
    fn convert_agent_tool_call_start() {
        let agent_event = AgentEvent::ToolCallStart {
            id: "t1".into(),
            name: "bash".into(),
        };
        let server_event: Option<ServerEvent> = (&agent_event).into();
        assert!(matches!(
            server_event,
            Some(ServerEvent::ToolCallStart { .. })
        ));
    }

    #[test]
    fn convert_agent_done() {
        let agent_event = AgentEvent::Done {
            total_usage: Usage {
                input_tokens: 100,
                output_tokens: 50,
            },
        };
        let server_event: Option<ServerEvent> = (&agent_event).into();
        assert!(matches!(server_event, Some(ServerEvent::Done { .. })));
    }

    #[test]
    fn convert_agent_turn_complete_is_none() {
        let agent_event = AgentEvent::TurnComplete {
            usage: Usage {
                input_tokens: 0,
                output_tokens: 0,
            },
        };
        let server_event: Option<ServerEvent> = (&agent_event).into();
        assert!(server_event.is_none());
    }
}

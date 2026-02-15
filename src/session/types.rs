use serde::{Deserialize, Serialize};

use crate::model::message::Message;

/// A unique session identifier (ULID).
pub type SessionId = String;

/// Each line in the JSONL file is one SessionEntry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEntry {
    /// Written once at session creation.
    Header {
        session_id: SessionId,
        model: String,
        created_at: String,
    },
    /// Each message exchanged (user, assistant, tool results).
    Message { message: Message },
    /// Usage stats per turn.
    Usage {
        input_tokens: u32,
        output_tokens: u32,
    },
}

/// Metadata for listing sessions (derived from Header entry).
#[derive(Debug, Clone)]
pub struct SessionMetadata {
    pub session_id: SessionId,
    pub model: String,
    pub created_at: String,
    pub message_count: usize,
    pub file_path: std::path::PathBuf,
}

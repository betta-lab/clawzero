use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use crate::error::ClawError;
use crate::model::message::Message;
use crate::session::types::{SessionEntry, SessionId, SessionMetadata};

/// Manages session storage on disk.
pub struct SessionStore {
    sessions_dir: PathBuf,
}

impl SessionStore {
    /// Create a new SessionStore. Creates the sessions directory if needed.
    pub fn new() -> Result<Self, ClawError> {
        let sessions_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("clawzero")
            .join("sessions");
        fs::create_dir_all(&sessions_dir).map_err(|e| {
            ClawError::Session(format!(
                "Failed to create sessions dir {}: {e}",
                sessions_dir.display()
            ))
        })?;
        Ok(Self { sessions_dir })
    }

    /// Create with a custom directory (for testing).
    pub fn with_dir(sessions_dir: PathBuf) -> Result<Self, ClawError> {
        fs::create_dir_all(&sessions_dir).map_err(|e| {
            ClawError::Session(format!(
                "Failed to create sessions dir {}: {e}",
                sessions_dir.display()
            ))
        })?;
        Ok(Self { sessions_dir })
    }

    /// Create a new session and return a writer for it.
    pub fn create_session(&self, model: &str) -> Result<SessionWriter, ClawError> {
        let session_id = ulid::Ulid::new().to_string().to_lowercase();
        let file_path = self.sessions_dir.join(format!("{session_id}.jsonl"));
        let file = File::create(&file_path)
            .map_err(|e| ClawError::Session(format!("Failed to create session file: {e}")))?;
        let mut writer = SessionWriter {
            writer: BufWriter::new(file),
            session_id: session_id.clone(),
        };

        let header = SessionEntry::Header {
            session_id: session_id.clone(),
            model: model.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        writer.append(&header)?;

        Ok(writer)
    }

    /// List all sessions, sorted by creation time (newest first).
    pub fn list_sessions(&self) -> Result<Vec<SessionMetadata>, ClawError> {
        let mut sessions = Vec::new();

        let entries = fs::read_dir(&self.sessions_dir)
            .map_err(|e| ClawError::Session(format!("Failed to read sessions dir: {e}")))?;

        for entry in entries {
            let entry = entry.map_err(|e| ClawError::Session(e.to_string()))?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "jsonl") {
                if let Some(meta) = self.read_session_metadata(&path) {
                    sessions.push(meta);
                }
            }
        }

        // Sort newest first
        sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(sessions)
    }

    /// Load all entries from a session.
    pub fn load_session(&self, session_id: &str) -> Result<Vec<SessionEntry>, ClawError> {
        let file_path = self.sessions_dir.join(format!("{session_id}.jsonl"));
        if !file_path.exists() {
            return Err(ClawError::Session(format!(
                "Session not found: {session_id}"
            )));
        }

        let file = File::open(&file_path)
            .map_err(|e| ClawError::Session(format!("Failed to open session: {e}")))?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|e| ClawError::Session(format!("Read error: {e}")))?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: SessionEntry = serde_json::from_str(&line)
                .map_err(|e| ClawError::Session(format!("Parse error: {e}")))?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Resume a session: returns a writer (in append mode) and the message history.
    pub fn resume_session(
        &self,
        session_id: &str,
    ) -> Result<(SessionWriter, Vec<Message>), ClawError> {
        let entries = self.load_session(session_id)?;

        let messages: Vec<Message> = entries
            .iter()
            .filter_map(|e| match e {
                SessionEntry::Message { message } => Some(message.clone()),
                _ => None,
            })
            .collect();

        let file_path = self.sessions_dir.join(format!("{session_id}.jsonl"));
        let file = OpenOptions::new()
            .append(true)
            .open(&file_path)
            .map_err(|e| ClawError::Session(format!("Failed to open session for append: {e}")))?;

        let writer = SessionWriter {
            writer: BufWriter::new(file),
            session_id: session_id.to_string(),
        };

        Ok((writer, messages))
    }

    fn read_session_metadata(&self, path: &std::path::Path) -> Option<SessionMetadata> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        let mut message_count = 0usize;
        let mut header_data: Option<(String, String, String)> = None;

        for line in reader.lines() {
            let line = line.ok()?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<SessionEntry>(&line) {
                match entry {
                    SessionEntry::Header {
                        session_id,
                        model,
                        created_at,
                    } => {
                        header_data = Some((session_id, model, created_at));
                    }
                    SessionEntry::Message { .. } => {
                        message_count += 1;
                    }
                    SessionEntry::Usage { .. } => {}
                }
            }
        }

        let (session_id, model, created_at) = header_data?;
        Some(SessionMetadata {
            session_id,
            model,
            created_at,
            message_count,
            file_path: path.to_path_buf(),
        })
    }
}

/// Appends entries to a session JSONL file.
pub struct SessionWriter {
    writer: BufWriter<File>,
    session_id: SessionId,
}

impl SessionWriter {
    /// Append an entry to the session file, flushing immediately.
    pub fn append(&mut self, entry: &SessionEntry) -> Result<(), ClawError> {
        let line = serde_json::to_string(entry)
            .map_err(|e| ClawError::Session(format!("Serialize error: {e}")))?;
        writeln!(self.writer, "{line}")
            .map_err(|e| ClawError::Session(format!("Write error: {e}")))?;
        self.writer
            .flush()
            .map_err(|e| ClawError::Session(format!("Flush error: {e}")))?;
        Ok(())
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::message::{ContentBlock, Message, Role};

    #[test]
    fn test_session_entry_roundtrip() {
        let entry = SessionEntry::Header {
            session_id: "test123".to_string(),
            model: "anthropic/claude-sonnet-4-20250514".to_string(),
            created_at: "2026-02-16T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: SessionEntry = serde_json::from_str(&json).unwrap();
        match parsed {
            SessionEntry::Header { session_id, .. } => assert_eq!(session_id, "test123"),
            _ => panic!("Expected Header"),
        }

        let msg_entry = SessionEntry::Message {
            message: Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "hello".to_string(),
                }],
            },
        };
        let json = serde_json::to_string(&msg_entry).unwrap();
        let parsed: SessionEntry = serde_json::from_str(&json).unwrap();
        match parsed {
            SessionEntry::Message { message } => {
                assert_eq!(message.role, Role::User);
            }
            _ => panic!("Expected Message"),
        }
    }

    #[test]
    fn test_create_and_load_session() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::with_dir(dir.path().to_path_buf()).unwrap();

        let mut writer = store.create_session("test/model").unwrap();
        let session_id = writer.session_id().to_string();

        let msg = SessionEntry::Message {
            message: Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "hello".to_string(),
                }],
            },
        };
        writer.append(&msg).unwrap();

        let entries = store.load_session(&session_id).unwrap();
        assert_eq!(entries.len(), 2); // header + message
        match &entries[0] {
            SessionEntry::Header { model, .. } => assert_eq!(model, "test/model"),
            _ => panic!("Expected Header"),
        }
    }

    #[test]
    fn test_list_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::with_dir(dir.path().to_path_buf()).unwrap();

        store.create_session("model-a").unwrap();
        store.create_session("model-b").unwrap();

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_resume_session() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::with_dir(dir.path().to_path_buf()).unwrap();

        let mut writer = store.create_session("test/model").unwrap();
        let session_id = writer.session_id().to_string();

        let msg = SessionEntry::Message {
            message: Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "hello".to_string(),
                }],
            },
        };
        writer.append(&msg).unwrap();
        drop(writer);

        let (mut writer, messages) = store.resume_session(&session_id).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, Role::User);

        // Can continue writing
        let msg2 = SessionEntry::Message {
            message: Message {
                role: Role::Assistant,
                content: vec![ContentBlock::Text {
                    text: "hi there".to_string(),
                }],
            },
        };
        writer.append(&msg2).unwrap();
        drop(writer);

        let entries = store.load_session(&session_id).unwrap();
        assert_eq!(entries.len(), 3); // header + 2 messages
    }

    #[test]
    fn test_empty_sessions_dir() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::with_dir(dir.path().to_path_buf()).unwrap();
        let sessions = store.list_sessions().unwrap();
        assert!(sessions.is_empty());
    }
}

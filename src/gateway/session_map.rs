use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

use crate::error::ClawError;

/// Key identifying a thread across platforms.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ThreadKey {
    pub platform: String,
    pub thread_id: String,
}

impl ThreadKey {
    fn to_map_key(&self) -> String {
        format!("{}:{}", self.platform, self.thread_id)
    }
}

/// Maps platform thread IDs to clawzero session IDs.
/// Persisted as JSON for cross-restart durability.
pub struct SessionMap {
    path: PathBuf,
    cache: RwLock<HashMap<String, String>>,
}

impl SessionMap {
    /// Create a SessionMap with default path (~/.local/share/clawzero/gateway_sessions.json).
    pub fn new() -> Result<Self, ClawError> {
        let path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("clawzero")
            .join("gateway_sessions.json");
        Self::with_path(path)
    }

    /// Create a SessionMap with a custom path (for testing).
    pub fn with_path(path: PathBuf) -> Result<Self, ClawError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ClawError::Gateway(format!("Failed to create session map dir: {e}"))
            })?;
        }

        let cache = if path.exists() {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| ClawError::Gateway(format!("Failed to read session map: {e}")))?;
            if content.trim().is_empty() {
                HashMap::new()
            } else {
                serde_json::from_str(&content).map_err(|e| {
                    ClawError::Gateway(format!("Failed to parse session map: {e}"))
                })?
            }
        } else {
            HashMap::new()
        };

        Ok(Self {
            path,
            cache: RwLock::new(cache),
        })
    }

    /// Get the session ID for a thread key.
    pub fn get(&self, key: &ThreadKey) -> Option<String> {
        self.cache.read().unwrap().get(&key.to_map_key()).cloned()
    }

    /// Store a mapping and persist to disk.
    pub fn put(&self, key: &ThreadKey, session_id: String) -> Result<(), ClawError> {
        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(key.to_map_key(), session_id);
        }
        self.persist()
    }

    /// Remove a mapping and persist to disk.
    pub fn remove(&self, key: &ThreadKey) -> Result<(), ClawError> {
        {
            let mut cache = self.cache.write().unwrap();
            cache.remove(&key.to_map_key());
        }
        self.persist()
    }

    fn persist(&self) -> Result<(), ClawError> {
        let cache = self.cache.read().unwrap();
        let json = serde_json::to_string_pretty(&*cache)
            .map_err(|e| ClawError::Gateway(format!("Failed to serialize session map: {e}")))?;
        std::fs::write(&self.path, json)
            .map_err(|e| ClawError::Gateway(format!("Failed to write session map: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_map() -> (tempfile::TempDir, SessionMap) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.json");
        let map = SessionMap::with_path(path).unwrap();
        (dir, map)
    }

    fn key(platform: &str, thread_id: &str) -> ThreadKey {
        ThreadKey {
            platform: platform.into(),
            thread_id: thread_id.into(),
        }
    }

    #[test]
    fn session_map_put_and_get() {
        let (_dir, map) = temp_map();
        let k = key("slack", "C123:1234.5678");
        map.put(&k, "session-abc".into()).unwrap();
        assert_eq!(map.get(&k), Some("session-abc".into()));
    }

    #[test]
    fn session_map_persistence_across_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.json");

        let k = key("discord", "guild:channel:thread");
        {
            let map = SessionMap::with_path(path.clone()).unwrap();
            map.put(&k, "session-xyz".into()).unwrap();
        }

        // Reload from disk
        let map2 = SessionMap::with_path(path).unwrap();
        assert_eq!(map2.get(&k), Some("session-xyz".into()));
    }

    #[test]
    fn session_map_remove() {
        let (_dir, map) = temp_map();
        let k = key("slack", "C123:ts");
        map.put(&k, "session-1".into()).unwrap();
        assert!(map.get(&k).is_some());

        map.remove(&k).unwrap();
        assert!(map.get(&k).is_none());
    }

    #[test]
    fn session_map_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.json");
        std::fs::write(&path, "").unwrap();

        let map = SessionMap::with_path(path).unwrap();
        assert!(map.get(&key("slack", "any")).is_none());
    }
}

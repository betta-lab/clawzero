use std::path::PathBuf;

use crate::error::ClawError;

/// Manages persistent memory stored in MEMORY.md files.
pub struct MemoryStore {
    /// Global memory: ~/.config/clawzero/MEMORY.md
    global_path: PathBuf,
    /// Project-local memory: ./.clawzero/MEMORY.md
    project_path: Option<PathBuf>,
}

impl MemoryStore {
    pub fn new() -> Self {
        let global_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("clawzero")
            .join("MEMORY.md");

        let project_path = find_project_root().map(|root| root.join(".clawzero").join("MEMORY.md"));

        Self {
            global_path,
            project_path,
        }
    }

    /// Create with custom paths (for testing).
    pub fn with_paths(global_path: PathBuf, project_path: Option<PathBuf>) -> Self {
        Self {
            global_path,
            project_path,
        }
    }

    /// Read all memory content (global + project, concatenated with headers).
    pub fn read_all(&self) -> String {
        let mut content = String::new();

        if let Ok(global) = std::fs::read_to_string(&self.global_path) {
            if !global.trim().is_empty() {
                content.push_str("# Global Memory\n\n");
                content.push_str(&global);
                content.push_str("\n\n");
            }
        }

        if let Some(ref project_path) = self.project_path {
            if let Ok(project) = std::fs::read_to_string(project_path) {
                if !project.trim().is_empty() {
                    content.push_str("# Project Memory\n\n");
                    content.push_str(&project);
                }
            }
        }

        content
    }

    /// Write to global memory (replaces entire content).
    pub fn write_global(&self, content: &str) -> Result<(), ClawError> {
        if let Some(parent) = self.global_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ClawError::Session(format!("Failed to create memory dir: {e}")))?;
        }
        std::fs::write(&self.global_path, content)
            .map_err(|e| ClawError::Session(format!("Failed to write global memory: {e}")))
    }

    /// Write to project memory (replaces entire content).
    pub fn write_project(&self, content: &str) -> Result<(), ClawError> {
        let path = self
            .project_path
            .as_ref()
            .ok_or_else(|| ClawError::Session("No project root found".to_string()))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ClawError::Session(format!("Failed to create project memory dir: {e}"))
            })?;
        }
        std::fs::write(path, content)
            .map_err(|e| ClawError::Session(format!("Failed to write project memory: {e}")))
    }
}

/// Walk up from cwd looking for a project root marker (.git, Cargo.toml).
fn find_project_root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let mut dir = cwd.as_path();
    loop {
        if dir.join(".git").exists() || dir.join("Cargo.toml").exists() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_empty_memory() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::with_paths(
            dir.path().join("global_MEMORY.md"),
            Some(dir.path().join("project_MEMORY.md")),
        );
        let content = store.read_all();
        assert!(content.is_empty());
    }

    #[test]
    fn test_write_and_read_global() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::with_paths(
            dir.path().join("global_MEMORY.md"),
            Some(dir.path().join("project_MEMORY.md")),
        );
        store.write_global("Test memory content").unwrap();
        let content = store.read_all();
        assert!(content.contains("Global Memory"));
        assert!(content.contains("Test memory content"));
    }

    #[test]
    fn test_write_and_read_project() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::with_paths(
            dir.path().join("global_MEMORY.md"),
            Some(dir.path().join("project_MEMORY.md")),
        );
        store.write_project("Project notes").unwrap();
        let content = store.read_all();
        assert!(content.contains("Project Memory"));
        assert!(content.contains("Project notes"));
    }

    #[test]
    fn test_read_all_combines_both() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::with_paths(
            dir.path().join("global_MEMORY.md"),
            Some(dir.path().join("project_MEMORY.md")),
        );
        store.write_global("Global info").unwrap();
        store.write_project("Project info").unwrap();
        let content = store.read_all();
        assert!(content.contains("Global info"));
        assert!(content.contains("Project info"));
        assert!(content.contains("# Global Memory"));
        assert!(content.contains("# Project Memory"));
    }
}

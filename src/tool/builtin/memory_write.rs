use std::sync::Arc;

use serde_json::json;

use crate::memory::store::MemoryStore;
use crate::model::tool_schema::ToolDefinition;
use crate::tool::traits::{Tool, ToolOutput};

pub struct MemoryWriteTool {
    store: Arc<MemoryStore>,
}

impl MemoryWriteTool {
    pub fn create(store: Arc<MemoryStore>) -> Arc<dyn Tool> {
        Arc::new(Self { store })
    }
}

impl Tool for MemoryWriteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "memory_write".to_string(),
            description: "Write to persistent memory (MEMORY.md). Use this to store information that should persist across sessions.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "scope": {
                        "type": "string",
                        "description": "Where to write: 'global' (user-wide) or 'project' (project-local)",
                        "enum": ["global", "project"]
                    },
                    "content": {
                        "type": "string",
                        "description": "The markdown content to write to MEMORY.md (replaces entire file)"
                    }
                },
                "required": ["scope", "content"]
            }),
        }
    }

    fn execute(
        &self,
        input: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
            let scope = match input["scope"].as_str() {
                Some(s) => s,
                None => {
                    return ToolOutput {
                        content: "Missing 'scope' parameter (must be 'global' or 'project')"
                            .to_string(),
                        is_error: true,
                    };
                }
            };

            let content = match input["content"].as_str() {
                Some(c) => c,
                None => {
                    return ToolOutput {
                        content: "Missing 'content' parameter".to_string(),
                        is_error: true,
                    };
                }
            };

            let result = match scope {
                "global" => self.store.write_global(content),
                "project" => self.store.write_project(content),
                _ => {
                    return ToolOutput {
                        content: format!("Invalid scope '{scope}', must be 'global' or 'project'"),
                        is_error: true,
                    };
                }
            };

            match result {
                Ok(()) => ToolOutput {
                    content: format!("Memory written to {scope} scope."),
                    is_error: false,
                },
                Err(e) => ToolOutput {
                    content: format!("Failed to write memory: {e}"),
                    is_error: true,
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_write_tool_definition() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(MemoryStore::with_paths(dir.path().join("MEMORY.md"), None));
        let tool = MemoryWriteTool::create(store);
        let def = tool.definition();
        assert_eq!(def.name, "memory_write");
        assert!(
            def.input_schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("scope"))
        );
    }

    #[tokio::test]
    async fn test_memory_write_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(MemoryStore::with_paths(
            dir.path().join("MEMORY.md"),
            Some(dir.path().join("project_MEMORY.md")),
        ));
        let tool = MemoryWriteTool::create(Arc::clone(&store));
        let output = tool
            .execute(json!({
                "scope": "global",
                "content": "# Test\nSome memory content"
            }))
            .await;
        assert!(!output.is_error);

        let content = store.read_all();
        assert!(content.contains("Test"));
        assert!(content.contains("Some memory content"));
    }
}

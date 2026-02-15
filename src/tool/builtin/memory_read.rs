use std::sync::Arc;

use serde_json::json;

use crate::memory::store::MemoryStore;
use crate::model::tool_schema::ToolDefinition;
use crate::tool::traits::{Tool, ToolOutput};

pub struct MemoryReadTool {
    store: Arc<MemoryStore>,
}

impl MemoryReadTool {
    pub fn new(store: Arc<MemoryStore>) -> Arc<dyn Tool> {
        Arc::new(Self { store })
    }
}

impl Tool for MemoryReadTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "memory_read".to_string(),
            description: "Read persistent memory (MEMORY.md). Returns both global and project-local memory content.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    fn execute(
        &self,
        _input: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
            let content = self.store.read_all();
            if content.is_empty() {
                ToolOutput {
                    content: "(no memory stored yet)".to_string(),
                    is_error: false,
                }
            } else {
                ToolOutput {
                    content,
                    is_error: false,
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_read_tool_definition() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(MemoryStore::with_paths(
            dir.path().join("MEMORY.md"),
            None,
        ));
        let tool = MemoryReadTool::new(store);
        let def = tool.definition();
        assert_eq!(def.name, "memory_read");
    }
}

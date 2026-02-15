use std::sync::Arc;

use serde_json::json;

use crate::model::tool_schema::ToolDefinition;
use crate::tool::traits::{Tool, ToolOutput};

pub struct FileWriteTool;

impl FileWriteTool {
    pub fn new() -> Arc<dyn Tool> {
        Arc::new(Self)
    }
}

impl Tool for FileWriteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "file_write".to_string(),
            description: "Write content to a file. Creates the file if it doesn't exist, or overwrites it."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path to write to"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    fn execute(
        &self,
        input: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
        let path = match input["path"].as_str() {
            Some(p) => p,
            None => {
                return ToolOutput {
                    content: "Missing 'path' parameter".to_string(),
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

        // Create parent directories if needed
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return ToolOutput {
                        content: format!("Failed to create directory '{}': {e}", parent.display()),
                        is_error: true,
                    };
                }
            }
        }

        match std::fs::write(path, content) {
            Ok(()) => {
                let bytes = content.len();
                ToolOutput {
                    content: format!("Wrote {bytes} bytes to {path}"),
                    is_error: false,
                }
            }
            Err(e) => ToolOutput {
                content: format!("Failed to write to '{path}': {e}"),
                is_error: true,
            },
        }
        })
    }
}

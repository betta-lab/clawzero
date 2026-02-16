use std::sync::Arc;

use serde_json::json;

use crate::model::tool_schema::ToolDefinition;
use crate::tool::traits::{Tool, ToolOutput};

pub struct FileEditTool;

impl FileEditTool {
    pub fn new() -> Arc<dyn Tool> {
        Arc::new(Self)
    }
}

impl Tool for FileEditTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "file_edit".to_string(),
            description:
                "Edit a file by replacing a specific text string with new text. The old_text must be unique in the file."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path to edit"
                    },
                    "old_text": {
                        "type": "string",
                        "description": "The exact text to find and replace (must be unique in the file)"
                    },
                    "new_text": {
                        "type": "string",
                        "description": "The text to replace it with"
                    }
                },
                "required": ["path", "old_text", "new_text"]
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

            let old_text = match input["old_text"].as_str() {
                Some(t) => t,
                None => {
                    return ToolOutput {
                        content: "Missing 'old_text' parameter".to_string(),
                        is_error: true,
                    };
                }
            };

            let new_text = match input["new_text"].as_str() {
                Some(t) => t,
                None => {
                    return ToolOutput {
                        content: "Missing 'new_text' parameter".to_string(),
                        is_error: true,
                    };
                }
            };

            let contents = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(e) => {
                    return ToolOutput {
                        content: format!("Failed to read '{path}': {e}"),
                        is_error: true,
                    };
                }
            };

            let count = contents.matches(old_text).count();

            if count == 0 {
                return ToolOutput {
                    content: format!("old_text not found in '{path}'"),
                    is_error: true,
                };
            }

            if count > 1 {
                return ToolOutput {
                    content: format!(
                        "old_text found {count} times in '{path}'. It must be unique. Provide more context."
                    ),
                    is_error: true,
                };
            }

            let new_contents = contents.replacen(old_text, new_text, 1);

            match std::fs::write(path, &new_contents) {
                Ok(()) => ToolOutput {
                    content: format!("Edited {path}"),
                    is_error: false,
                },
                Err(e) => ToolOutput {
                    content: format!("Failed to write '{path}': {e}"),
                    is_error: true,
                },
            }
        })
    }
}

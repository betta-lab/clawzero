use std::sync::Arc;

use serde_json::json;

use crate::model::tool_schema::ToolDefinition;
use crate::tool::traits::{Tool, ToolOutput};

pub struct FileReadTool;

impl FileReadTool {
    pub fn create() -> Arc<dyn Tool> {
        Arc::new(Self)
    }
}

impl Tool for FileReadTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "file_read".to_string(),
            description:
                "Read the contents of a file. Returns the file contents with line numbers."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path to read"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Line number to start reading from (1-based, default: 1)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of lines to read (default: 2000)"
                    }
                },
                "required": ["path"]
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

            let offset = input["offset"].as_u64().unwrap_or(1).max(1) as usize;
            let limit = input["limit"].as_u64().unwrap_or(2000) as usize;

            match std::fs::read_to_string(path) {
                Ok(contents) => {
                    let lines: Vec<&str> = contents.lines().collect();
                    let total_lines = lines.len();
                    let start = (offset - 1).min(total_lines);
                    let end = (start + limit).min(total_lines);

                    let mut output = String::new();
                    for (i, line) in lines[start..end].iter().enumerate() {
                        let line_num = start + i + 1;
                        output.push_str(&format!("{line_num:>6}\t{line}\n"));
                    }

                    if output.is_empty() {
                        output = "(empty file)".to_string();
                    }

                    ToolOutput {
                        content: output,
                        is_error: false,
                    }
                }
                Err(e) => ToolOutput {
                    content: format!("Failed to read file '{path}': {e}"),
                    is_error: true,
                },
            }
        })
    }
}

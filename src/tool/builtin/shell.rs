use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::process::Command;

use crate::model::tool_schema::ToolDefinition;
use crate::tool::traits::{Tool, ToolOutput};

pub struct ShellTool;

impl ShellTool {
    pub fn new() -> Arc<dyn Tool> {
        Arc::new(Self)
    }
}

impl Tool for ShellTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "bash".to_string(),
            description: "Execute a bash command and return stdout/stderr.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The bash command to execute"
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "description": "Timeout in milliseconds (default: 120000)"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    fn execute(
        &self,
        input: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
            let command = match input["command"].as_str() {
                Some(c) => c,
                None => {
                    return ToolOutput {
                        content: "Missing 'command' parameter".to_string(),
                        is_error: true,
                    };
                }
            };

            let timeout_ms = input["timeout_ms"].as_u64().unwrap_or(120_000);

            let result = tokio::time::timeout(
                Duration::from_millis(timeout_ms),
                Command::new("bash").arg("-c").arg(command).output(),
            )
            .await;

            match result {
                Ok(Ok(output)) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let mut content = String::new();
                    if !stdout.is_empty() {
                        content.push_str(&stdout);
                    }
                    if !stderr.is_empty() {
                        if !content.is_empty() {
                            content.push('\n');
                        }
                        content.push_str("[stderr]\n");
                        content.push_str(&stderr);
                    }
                    if content.is_empty() {
                        content = "(no output)".to_string();
                    }
                    ToolOutput {
                        content,
                        is_error: !output.status.success(),
                    }
                }
                Ok(Err(e)) => ToolOutput {
                    content: format!("Failed to execute command: {e}"),
                    is_error: true,
                },
                Err(_) => ToolOutput {
                    content: format!("Command timed out after {timeout_ms}ms"),
                    is_error: true,
                },
            }
        })
    }
}

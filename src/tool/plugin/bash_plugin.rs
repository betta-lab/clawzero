use std::sync::Arc;
use std::time::Duration;

use crate::model::tool_schema::ToolDefinition;
use crate::tool::plugin::types::{PluginToolConfig, substitute_template};
use crate::tool::traits::{Tool, ToolOutput};

pub struct BashPluginTool {
    config: PluginToolConfig,
}

impl BashPluginTool {
    pub fn create(config: PluginToolConfig) -> Arc<dyn Tool> {
        Arc::new(Self { config })
    }
}

impl Tool for BashPluginTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.config.name.clone(),
            description: self.config.description.clone(),
            input_schema: self.config.input_schema.clone().unwrap_or_else(|| {
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                })
            }),
        }
    }

    fn execute(
        &self,
        input: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
            let command_template = match &self.config.command {
                Some(cmd) => cmd,
                None => {
                    return ToolOutput {
                        content: "Plugin config missing 'command' field".to_string(),
                        is_error: true,
                    };
                }
            };

            let command = substitute_template(command_template, &input);
            let timeout = Duration::from_millis(self.config.timeout_ms.unwrap_or(120_000));

            match tokio::time::timeout(
                timeout,
                tokio::process::Command::new("bash")
                    .arg("-c")
                    .arg(&command)
                    .output(),
            )
            .await
            {
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
                        content.push_str("[stderr] ");
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
                    content: format!("Command timed out after {}ms", timeout.as_millis()),
                    is_error: true,
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::plugin::types::PluginType;

    fn make_config(command: &str) -> PluginToolConfig {
        PluginToolConfig {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            tool_type: PluginType::Bash,
            command: Some(command.to_string()),
            url: None,
            method: "GET".to_string(),
            headers: Default::default(),
            body_template: None,
            input_schema: None,
            timeout_ms: Some(5000),
        }
    }

    #[tokio::test]
    async fn test_bash_plugin_simple() {
        let tool = BashPluginTool::create(make_config("echo {{message}}"));
        let output = tool
            .execute(serde_json::json!({"message": "hello_plugin"}))
            .await;
        assert!(!output.is_error);
        assert!(output.content.contains("hello_plugin"));
    }

    #[tokio::test]
    async fn test_bash_plugin_no_params() {
        let tool = BashPluginTool::create(make_config("echo static_output"));
        let output = tool.execute(serde_json::json!({})).await;
        assert!(!output.is_error);
        assert!(output.content.contains("static_output"));
    }
}

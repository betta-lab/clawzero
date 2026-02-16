use std::sync::Arc;
use std::time::Duration;

use crate::model::tool_schema::ToolDefinition;
use crate::tool::plugin::types::{PluginToolConfig, expand_env_vars, substitute_template};
use crate::tool::traits::{Tool, ToolOutput};

pub struct HttpPluginTool {
    config: PluginToolConfig,
    client: reqwest::Client,
}

impl HttpPluginTool {
    pub fn create(config: PluginToolConfig) -> Arc<dyn Tool> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms.unwrap_or(30_000)))
            .build()
            .expect("Failed to build HTTP client for plugin");
        Arc::new(Self { config, client })
    }
}

impl Tool for HttpPluginTool {
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
            let url_template = match &self.config.url {
                Some(u) => u,
                None => {
                    return ToolOutput {
                        content: "Plugin config missing 'url' field".to_string(),
                        is_error: true,
                    };
                }
            };

            let url = substitute_template(url_template, &input);

            let method = self.config.method.to_uppercase();
            let mut builder = match method.as_str() {
                "GET" => self.client.get(&url),
                "POST" => self.client.post(&url),
                "PUT" => self.client.put(&url),
                "DELETE" => self.client.delete(&url),
                "PATCH" => self.client.patch(&url),
                _ => {
                    return ToolOutput {
                        content: format!("Unsupported HTTP method: {method}"),
                        is_error: true,
                    };
                }
            };

            // Apply headers with env var expansion
            for (key, value) in &self.config.headers {
                let expanded = expand_env_vars(value);
                builder = builder.header(key, expanded);
            }

            // Apply body template
            if let Some(body_template) = &self.config.body_template {
                let body = substitute_template(body_template, &input);
                builder = builder
                    .header("Content-Type", "application/json")
                    .body(body);
            }

            match builder.send().await {
                Ok(response) => {
                    let status = response.status();
                    match response.text().await {
                        Ok(body) => ToolOutput {
                            content: if status.is_success() {
                                body
                            } else {
                                format!("HTTP {status}: {body}")
                            },
                            is_error: !status.is_success(),
                        },
                        Err(e) => ToolOutput {
                            content: format!("Failed to read response body: {e}"),
                            is_error: true,
                        },
                    }
                }
                Err(e) => ToolOutput {
                    content: format!("HTTP request failed: {e}"),
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

    #[test]
    fn test_http_plugin_definition() {
        let config = PluginToolConfig {
            name: "test_api".to_string(),
            description: "Test API call".to_string(),
            tool_type: PluginType::Http,
            command: None,
            url: Some("https://example.com/api/{{resource}}".to_string()),
            method: "GET".to_string(),
            headers: Default::default(),
            body_template: None,
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "resource": {"type": "string"}
                }
            })),
            timeout_ms: None,
        };
        let tool = HttpPluginTool::create(config);
        let def = tool.definition();
        assert_eq!(def.name, "test_api");
    }

    #[test]
    fn test_http_plugin_template_substitution() {
        let url = "https://api.example.com/{{version}}/{{endpoint}}";
        let params = serde_json::json!({"version": "v1", "endpoint": "users"});
        let result = substitute_template(url, &params);
        assert_eq!(result, "https://api.example.com/v1/users");
    }
}

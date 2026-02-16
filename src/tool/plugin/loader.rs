use std::sync::Arc;

use crate::tool::plugin::bash_plugin::BashPluginTool;
use crate::tool::plugin::http_plugin::HttpPluginTool;
use crate::tool::plugin::types::{PluginToolConfig, PluginType};
use crate::tool::traits::Tool;

/// Load plugin tools from configuration. Returns a Vec of Tool trait objects.
pub fn load_plugin_tools(configs: &[PluginToolConfig]) -> Vec<Arc<dyn Tool>> {
    configs
        .iter()
        .map(|config| match config.tool_type {
            PluginType::Bash => BashPluginTool::create(config.clone()),
            PluginType::Http => HttpPluginTool::create(config.clone()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_empty_plugins() {
        let tools = load_plugin_tools(&[]);
        assert!(tools.is_empty());
    }

    #[test]
    fn test_load_bash_plugin() {
        let config = PluginToolConfig {
            name: "my_tool".to_string(),
            description: "My custom tool".to_string(),
            tool_type: PluginType::Bash,
            command: Some("echo test".to_string()),
            url: None,
            method: "GET".to_string(),
            headers: Default::default(),
            body_template: None,
            input_schema: None,
            timeout_ms: None,
        };
        let tools = load_plugin_tools(&[config]);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].definition().name, "my_tool");
    }

    #[test]
    fn test_load_http_plugin() {
        let config = PluginToolConfig {
            name: "api_call".to_string(),
            description: "Call an API".to_string(),
            tool_type: PluginType::Http,
            command: None,
            url: Some("https://example.com".to_string()),
            method: "POST".to_string(),
            headers: Default::default(),
            body_template: Some(r#"{"key": "{{value}}"}"#.to_string()),
            input_schema: None,
            timeout_ms: None,
        };
        let tools = load_plugin_tools(&[config]);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].definition().name, "api_call");
    }
}

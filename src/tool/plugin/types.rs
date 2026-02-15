use std::collections::HashMap;

use serde::Deserialize;

/// Configuration for a plugin tool defined in TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct PluginToolConfig {
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub tool_type: PluginType,
    /// Shell command template (for bash type). Supports {{param}} placeholders.
    pub command: Option<String>,
    /// URL template (for http type). Supports {{param}} placeholders.
    pub url: Option<String>,
    /// HTTP method (for http type, default: GET).
    #[serde(default = "default_method")]
    pub method: String,
    /// HTTP headers. Supports ${ENV_VAR} expansion.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// HTTP body template. Supports {{param}} placeholders.
    pub body_template: Option<String>,
    /// JSON Schema for tool input parameters.
    pub input_schema: Option<serde_json::Value>,
    /// Timeout in milliseconds.
    pub timeout_ms: Option<u64>,
}

fn default_method() -> String {
    "GET".to_string()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    Bash,
    Http,
}

/// Substitute {{param}} placeholders in a template string with values from a JSON object.
pub fn substitute_template(template: &str, params: &serde_json::Value) -> String {
    let mut result = template.to_string();
    if let Some(obj) = params.as_object() {
        for (key, value) in obj {
            let placeholder = format!("{{{{{key}}}}}");
            let replacement = match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            result = result.replace(&placeholder, &replacement);
        }
    }
    result
}

/// Expand ${ENV_VAR} references in a string with environment variable values.
pub fn expand_env_vars(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'
            let mut var_name = String::new();
            for c in chars.by_ref() {
                if c == '}' {
                    break;
                }
                var_name.push(c);
            }
            if let Ok(val) = std::env::var(&var_name) {
                result.push_str(&val);
            }
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_template() {
        let template = "git log --oneline -{{count}}";
        let params = serde_json::json!({"count": 10});
        assert_eq!(substitute_template(template, &params), "git log --oneline -10");
    }

    #[test]
    fn test_substitute_template_string() {
        let template = "echo {{message}}";
        let params = serde_json::json!({"message": "hello world"});
        assert_eq!(substitute_template(template, &params), "echo hello world");
    }

    #[test]
    fn test_expand_env_vars() {
        // SAFETY: single-threaded test
        unsafe { std::env::set_var("CLAWZERO_TEST_VAR", "test_value") };
        let result = expand_env_vars("Bearer ${CLAWZERO_TEST_VAR}");
        assert_eq!(result, "Bearer test_value");
        unsafe { std::env::remove_var("CLAWZERO_TEST_VAR") };
    }

    #[test]
    fn test_expand_env_vars_missing() {
        let result = expand_env_vars("${NONEXISTENT_VAR_12345}");
        assert_eq!(result, "");
    }

    #[test]
    fn test_deserialize_plugin_config() {
        let toml_str = r#"
name = "git_diff"
description = "Show git diff"
type = "bash"
command = "git diff {{args}}"
"#;
        let config: PluginToolConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.name, "git_diff");
        assert!(matches!(config.tool_type, PluginType::Bash));
        assert_eq!(config.command.unwrap(), "git diff {{args}}");
    }
}

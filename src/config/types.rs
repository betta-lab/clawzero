use std::collections::HashMap;

use serde::Deserialize;

use crate::tool::plugin::types::PluginToolConfig;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    /// Plugin tool definitions.
    #[serde(default)]
    pub tools: Vec<PluginToolConfig>,
    /// Gateway configuration for Slack/Discord bots.
    #[serde(default)]
    pub gateway: GatewayConfig,
}

#[derive(Debug, Default, Deserialize)]
pub struct GatewayConfig {
    pub slack: Option<SlackConfig>,
    pub discord: Option<DiscordConfig>,
    pub webui: Option<WebuiConfig>,
}

#[derive(Debug, Deserialize)]
pub struct SlackConfig {
    /// Socket Mode app token (xapp-...).
    pub app_token: Option<String>,
    /// Env var name for app token.
    pub app_token_env: Option<String>,
    /// Bot token (xoxb-...).
    pub bot_token: Option<String>,
    /// Env var name for bot token.
    pub bot_token_env: Option<String>,
}

impl SlackConfig {
    /// Resolve app token from direct value or env var.
    pub fn resolve_app_token(&self) -> Option<String> {
        self.app_token.clone().or_else(|| {
            self.app_token_env
                .as_ref()
                .and_then(|env| std::env::var(env).ok())
        })
    }

    /// Resolve bot token from direct value or env var.
    pub fn resolve_bot_token(&self) -> Option<String> {
        self.bot_token.clone().or_else(|| {
            self.bot_token_env
                .as_ref()
                .and_then(|env| std::env::var(env).ok())
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct DiscordConfig {
    /// Discord bot token.
    pub bot_token: Option<String>,
    /// Env var name for bot token.
    pub bot_token_env: Option<String>,
}

impl DiscordConfig {
    /// Resolve bot token from direct value or env var.
    pub fn resolve_bot_token(&self) -> Option<String> {
        self.bot_token.clone().or_else(|| {
            self.bot_token_env
                .as_ref()
                .and_then(|env| std::env::var(env).ok())
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct WebuiConfig {
    /// Bind host (default: "127.0.0.1").
    pub host: Option<String>,
    /// Bind port (default: 3000).
    pub port: Option<u16>,
}

impl WebuiConfig {
    pub fn host(&self) -> &str {
        self.host.as_deref().unwrap_or("127.0.0.1")
    }

    pub fn port(&self) -> u16 {
        self.port.unwrap_or(3000)
    }
}

#[derive(Debug, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default = "DefaultsConfig::default_model")]
    pub model: String,
    #[serde(default = "DefaultsConfig::default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "DefaultsConfig::default_max_turns")]
    pub max_turns: usize,
    /// Context window limit in tokens (default: 200000).
    #[serde(default = "DefaultsConfig::default_context_limit")]
    pub context_limit: u32,
}

impl DefaultsConfig {
    fn default_model() -> String {
        "anthropic/claude-sonnet-4-20250514".to_string()
    }

    fn default_max_tokens() -> u32 {
        8192
    }

    fn default_max_turns() -> usize {
        25
    }

    fn default_context_limit() -> u32 {
        200_000
    }
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            model: Self::default_model(),
            max_tokens: Self::default_max_tokens(),
            max_turns: Self::default_max_turns(),
            context_limit: Self::default_context_limit(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ProviderConfig {
    pub protocol: ProtocolType,
    pub base_url: String,
    pub api_key: Option<String>,
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub extra_headers: HashMap<String, String>,
    #[serde(default)]
    pub models: Vec<String>,
    /// Authentication method: "vertex" or "bedrock".
    pub auth: Option<AuthType>,
    /// GCP project ID (for Vertex AI).
    pub project_id: Option<String>,
    /// Cloud region (for Vertex AI / Bedrock).
    pub region: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProtocolType {
    Anthropic,
    Openai,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    Vertex,
    Bedrock,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_with_gateway_parses() {
        let toml_str = r#"
[gateway.slack]
app_token = "xapp-test"
bot_token = "xoxb-test"

[gateway.discord]
bot_token = "discord-test"
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        let slack = config.gateway.slack.unwrap();
        assert_eq!(slack.app_token.unwrap(), "xapp-test");
        assert_eq!(slack.bot_token.unwrap(), "xoxb-test");
        let discord = config.gateway.discord.unwrap();
        assert_eq!(discord.bot_token.unwrap(), "discord-test");
    }

    #[test]
    fn config_without_gateway_defaults() {
        let toml_str = r#"
[defaults]
max_tokens = 4096
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert!(config.gateway.slack.is_none());
        assert!(config.gateway.discord.is_none());
        assert!(config.gateway.webui.is_none());
    }

    #[test]
    fn config_with_webui_gateway_parses() {
        let toml_str = r#"
[gateway.webui]
host = "0.0.0.0"
port = 8080
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        let webui = config.gateway.webui.unwrap();
        assert_eq!(webui.host(), "0.0.0.0");
        assert_eq!(webui.port(), 8080);
    }

    #[test]
    fn webui_config_defaults() {
        let toml_str = r#"
[gateway.webui]
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        let webui = config.gateway.webui.unwrap();
        assert_eq!(webui.host(), "127.0.0.1");
        assert_eq!(webui.port(), 3000);
    }

    #[test]
    fn gateway_error_display() {
        let err = crate::error::ClawError::Gateway("test error".into());
        assert_eq!(err.to_string(), "Gateway error: test error");
        let err = crate::error::ClawError::WebSocket("ws error".into());
        assert_eq!(err.to_string(), "WebSocket error: ws error");
    }
}

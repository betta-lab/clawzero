use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
}

#[derive(Debug, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default = "DefaultsConfig::default_model")]
    pub model: String,
    #[serde(default = "DefaultsConfig::default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "DefaultsConfig::default_max_turns")]
    pub max_turns: usize,
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
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            model: Self::default_model(),
            max_tokens: Self::default_max_tokens(),
            max_turns: Self::default_max_turns(),
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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProtocolType {
    Anthropic,
    Openai,
}

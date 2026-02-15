use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::types::{AppConfig, ProtocolType, ProviderConfig};
use crate::error::ClawError;

pub fn load_config() -> Result<AppConfig, ClawError> {
    let mut config = builtin_defaults();

    // Load global config: ~/.config/clawzero/config.toml
    if let Some(global_path) = global_config_path() {
        if global_path.exists() {
            let content = std::fs::read_to_string(&global_path)
                .map_err(|e| ClawError::Config(format!("Failed to read {}: {e}", global_path.display())))?;
            let file_config: AppConfig = toml::from_str(&content)
                .map_err(|e| ClawError::Config(format!("Failed to parse {}: {e}", global_path.display())))?;
            merge_config(&mut config, file_config);
        }
    }

    // Load project-local config: ./clawzero.toml
    let local_path = PathBuf::from("clawzero.toml");
    if local_path.exists() {
        let content = std::fs::read_to_string(&local_path)
            .map_err(|e| ClawError::Config(format!("Failed to read clawzero.toml: {e}")))?;
        let file_config: AppConfig = toml::from_str(&content)
            .map_err(|e| ClawError::Config(format!("Failed to parse clawzero.toml: {e}")))?;
        merge_config(&mut config, file_config);
    }

    // Override model from env
    if let Ok(model) = std::env::var("CLAWZERO_MODEL") {
        config.defaults.model = model;
    }

    Ok(config)
}

fn global_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("clawzero").join("config.toml"))
}

fn builtin_defaults() -> AppConfig {
    let mut providers = HashMap::new();

    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            protocol: ProtocolType::Anthropic,
            base_url: "https://api.anthropic.com".to_string(),
            api_key: None,
            api_key_env: Some("ANTHROPIC_API_KEY".to_string()),
            extra_headers: HashMap::new(),
            models: vec![
                "claude-opus-4-6".to_string(),
                "claude-sonnet-4-20250514".to_string(),
                "claude-haiku-35-20241022".to_string(),
            ],
            auth: None,
            project_id: None,
            region: None,
        },
    );

    providers.insert(
        "openai".to_string(),
        ProviderConfig {
            protocol: ProtocolType::Openai,
            base_url: "https://api.openai.com".to_string(),
            api_key: None,
            api_key_env: Some("OPENAI_API_KEY".to_string()),
            extra_headers: HashMap::new(),
            models: vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "o3-mini".to_string(),
            ],
            auth: None,
            project_id: None,
            region: None,
        },
    );

    AppConfig {
        defaults: Default::default(),
        providers,
        tools: Vec::new(),
    }
}

fn merge_config(base: &mut AppConfig, overlay: AppConfig) {
    // Overlay defaults override base defaults field-by-field
    // (serde defaults mean if a field is missing, it gets default value,
    //  so we just replace the whole defaults struct)
    base.defaults = overlay.defaults;

    // Merge providers: overlay providers override base providers by name
    for (name, provider) in overlay.providers {
        base.providers.insert(name, provider);
    }
}

/// Resolve the API key for a provider config.
/// Tries `api_key` first, then reads from env var specified by `api_key_env`.
pub fn resolve_api_key(config: &ProviderConfig) -> Result<String, ClawError> {
    if let Some(key) = &config.api_key {
        if !key.is_empty() {
            return Ok(key.clone());
        }
    }
    if let Some(env_var) = &config.api_key_env {
        if let Ok(key) = std::env::var(env_var) {
            if !key.is_empty() {
                return Ok(key);
            }
        }
        return Err(ClawError::Config(format!(
            "API key not found: set {env_var} environment variable"
        )));
    }
    // No key configured — might be fine for local providers like Ollama
    Ok(String::new())
}

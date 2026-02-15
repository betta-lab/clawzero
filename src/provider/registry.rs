use std::collections::HashMap;
use std::sync::Arc;

use crate::config::loader::resolve_api_key;
use crate::config::types::{AppConfig, AuthType, ProtocolType};
use crate::error::ClawError;
use crate::provider::auth::vertex::VertexAuth;
use crate::provider::auth::AuthHook;
use crate::provider::protocol::anthropic::AnthropicProtocol;
use crate::provider::protocol::openai::OpenAiProtocol;
use crate::provider::traits::Provider;

/// Holds all configured providers. Resolves "provider/model" specs.
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
}

impl ProviderRegistry {
    /// Build from application config. Creates one Provider instance per configured provider.
    /// Providers whose API keys are missing are skipped (lazy — error only on use).
    pub fn from_config(config: &AppConfig) -> Result<Self, ClawError> {
        let mut providers: HashMap<String, Arc<dyn Provider>> = HashMap::new();

        for (name, provider_config) in &config.providers {
            let api_key = resolve_api_key(provider_config).unwrap_or_default();

            // Build auth hook if configured
            let auth_hook: Option<Arc<dyn AuthHook>> = match &provider_config.auth {
                Some(AuthType::Vertex) => {
                    let project_id = provider_config
                        .project_id
                        .clone()
                        .or_else(|| std::env::var("GCLOUD_PROJECT").ok())
                        .or_else(|| std::env::var("GOOGLE_CLOUD_PROJECT").ok())
                        .ok_or_else(|| {
                            ClawError::Config(format!(
                                "Provider '{name}': Vertex AI requires project_id \
                                 (set in config or GCLOUD_PROJECT env var)"
                            ))
                        })?;
                    let region = provider_config
                        .region
                        .clone()
                        .unwrap_or_else(|| "us-central1".to_string());
                    Some(Arc::new(VertexAuth::new(project_id, region)))
                }
                #[cfg(feature = "bedrock")]
                Some(AuthType::Bedrock) => {
                    use crate::provider::auth::bedrock::BedrockAuth;
                    let auth = BedrockAuth::from_env(provider_config.region.clone())?;
                    Some(Arc::new(auth))
                }
                #[cfg(not(feature = "bedrock"))]
                Some(AuthType::Bedrock) => {
                    return Err(ClawError::Config(format!(
                        "Provider '{name}': Bedrock auth requires the 'bedrock' feature. \
                         Build with: cargo build --features bedrock"
                    )));
                }
                None => None,
            };

            let provider: Arc<dyn Provider> = match provider_config.protocol {
                ProtocolType::Anthropic => Arc::new(AnthropicProtocol::new(
                    provider_config.base_url.clone(),
                    api_key,
                    provider_config.extra_headers.clone(),
                    auth_hook,
                )),
                ProtocolType::Openai => Arc::new(OpenAiProtocol::new(
                    name.clone(),
                    provider_config.base_url.clone(),
                    api_key,
                    provider_config.extra_headers.clone(),
                    auth_hook,
                )),
            };

            providers.insert(name.clone(), provider);
        }

        Ok(Self { providers })
    }

    /// Resolve "provider/model" -> (provider instance, model name).
    pub fn resolve(&self, spec: &str) -> Result<(Arc<dyn Provider>, String), ClawError> {
        let (provider_name, model) = spec
            .split_once('/')
            .ok_or_else(|| ClawError::InvalidModelSpec(spec.to_string()))?;

        let provider = self
            .providers
            .get(provider_name)
            .ok_or_else(|| ClawError::UnknownProvider(provider_name.to_string()))?;

        Ok((Arc::clone(provider), model.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::{DefaultsConfig, ProviderConfig};

    #[test]
    fn test_registry_vertex_requires_project_id() {
        let mut providers = HashMap::new();
        providers.insert(
            "vertex".to_string(),
            ProviderConfig {
                protocol: ProtocolType::Anthropic,
                base_url: "https://placeholder".to_string(),
                api_key: Some("key".to_string()),
                api_key_env: None,
                extra_headers: HashMap::new(),
                models: vec![],
                auth: Some(AuthType::Vertex),
                project_id: None,
                region: None,
            },
        );
        let config = AppConfig {
            defaults: DefaultsConfig::default(),
            providers,
            tools: Vec::new(),
        };

        // Safety: test-only, no concurrent env var access expected
        unsafe {
            std::env::remove_var("GCLOUD_PROJECT");
            std::env::remove_var("GOOGLE_CLOUD_PROJECT");
        }

        match ProviderRegistry::from_config(&config) {
            Err(e) => {
                let err = e.to_string();
                assert!(err.contains("project_id"), "Error should mention project_id: {err}");
            }
            Ok(_) => panic!("Expected error for missing project_id"),
        }
    }

    #[test]
    fn test_registry_vertex_with_project_id() {
        let mut providers = HashMap::new();
        providers.insert(
            "vertex".to_string(),
            ProviderConfig {
                protocol: ProtocolType::Anthropic,
                base_url: "https://placeholder".to_string(),
                api_key: Some("key".to_string()),
                api_key_env: None,
                extra_headers: HashMap::new(),
                models: vec![],
                auth: Some(AuthType::Vertex),
                project_id: Some("my-gcp-project".to_string()),
                region: Some("us-central1".to_string()),
            },
        );
        let config = AppConfig {
            defaults: DefaultsConfig::default(),
            providers,
            tools: Vec::new(),
        };

        let registry = ProviderRegistry::from_config(&config).unwrap();
        let (provider, model) = registry.resolve("vertex/claude-sonnet-4-20250514").unwrap();
        assert_eq!(model, "claude-sonnet-4-20250514");
        assert_eq!(provider.name(), "anthropic");
    }

    #[cfg(not(feature = "bedrock"))]
    #[test]
    fn test_registry_bedrock_without_feature() {
        let mut providers = HashMap::new();
        providers.insert(
            "bedrock".to_string(),
            ProviderConfig {
                protocol: ProtocolType::Anthropic,
                base_url: "https://placeholder".to_string(),
                api_key: Some("key".to_string()),
                api_key_env: None,
                extra_headers: HashMap::new(),
                models: vec![],
                auth: Some(AuthType::Bedrock),
                project_id: None,
                region: None,
            },
        );
        let config = AppConfig {
            defaults: DefaultsConfig::default(),
            providers,
            tools: Vec::new(),
        };

        match ProviderRegistry::from_config(&config) {
            Err(e) => {
                let err = e.to_string();
                assert!(err.contains("bedrock"), "Error should mention bedrock feature: {err}");
            }
            Ok(_) => panic!("Expected error for bedrock without feature"),
        }
    }

    #[test]
    fn test_registry_no_auth() {
        let mut providers = HashMap::new();
        providers.insert(
            "openai".to_string(),
            ProviderConfig {
                protocol: ProtocolType::Openai,
                base_url: "https://api.openai.com".to_string(),
                api_key: Some("sk-test".to_string()),
                api_key_env: None,
                extra_headers: HashMap::new(),
                models: vec![],
                auth: None,
                project_id: None,
                region: None,
            },
        );
        let config = AppConfig {
            defaults: DefaultsConfig::default(),
            providers,
            tools: Vec::new(),
        };

        let registry = ProviderRegistry::from_config(&config).unwrap();
        let (provider, model) = registry.resolve("openai/gpt-4o").unwrap();
        assert_eq!(model, "gpt-4o");
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_config_auth_type_deserialize() {
        let toml_str = r#"
[defaults]

[providers.vertex-claude]
protocol = "anthropic"
base_url = "https://us-central1-aiplatform.googleapis.com"
auth = "vertex"
project_id = "my-project"
region = "us-central1"
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        let provider = config.providers.get("vertex-claude").unwrap();
        assert!(matches!(provider.auth, Some(AuthType::Vertex)));
        assert_eq!(provider.project_id.as_deref(), Some("my-project"));
        assert_eq!(provider.region.as_deref(), Some("us-central1"));
    }
}

use std::collections::HashMap;
use std::sync::Arc;

use crate::config::loader::resolve_api_key;
use crate::config::types::{AppConfig, ProtocolType};
use crate::error::ClawError;
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

            let provider: Arc<dyn Provider> = match provider_config.protocol {
                ProtocolType::Anthropic => Arc::new(AnthropicProtocol::new(
                    provider_config.base_url.clone(),
                    api_key,
                    provider_config.extra_headers.clone(),
                )),
                ProtocolType::Openai => Arc::new(OpenAiProtocol::new(
                    name.clone(),
                    provider_config.base_url.clone(),
                    api_key,
                    provider_config.extra_headers.clone(),
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

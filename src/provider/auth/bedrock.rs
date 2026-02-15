//! AWS Bedrock authentication using SigV4 request signing.
//!
//! This module is only available when the `bedrock` feature is enabled.

use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::error::ClawError;
use crate::provider::auth::AuthHook;

/// Bedrock authentication using AWS SigV4.
pub struct BedrockAuth {
    region: String,
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
}

impl BedrockAuth {
    /// Create from explicit credentials.
    pub fn new(
        region: String,
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
    ) -> Self {
        Self {
            region,
            access_key_id,
            secret_access_key,
            session_token,
        }
    }

    /// Create from environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION).
    pub fn from_env(region: Option<String>) -> Result<Self, ClawError> {
        let region = region
            .or_else(|| std::env::var("AWS_REGION").ok())
            .or_else(|| std::env::var("AWS_DEFAULT_REGION").ok())
            .unwrap_or_else(|| "us-east-1".to_string());

        let access_key_id = std::env::var("AWS_ACCESS_KEY_ID")
            .map_err(|_| ClawError::Auth("AWS_ACCESS_KEY_ID not set".to_string()))?;
        let secret_access_key = std::env::var("AWS_SECRET_ACCESS_KEY")
            .map_err(|_| ClawError::Auth("AWS_SECRET_ACCESS_KEY not set".to_string()))?;
        let session_token = std::env::var("AWS_SESSION_TOKEN").ok();

        Ok(Self::new(
            region,
            access_key_id,
            secret_access_key,
            session_token,
        ))
    }

    /// Get Bedrock endpoint URL for a model.
    pub fn endpoint_url(&self, model: &str) -> String {
        format!(
            "https://bedrock-runtime.{region}.amazonaws.com/model/{model}/invoke-with-response-stream",
            region = self.region,
        )
    }
}

impl AuthHook for BedrockAuth {
    fn prepare_request<'a>(
        &'a self,
        builder: reqwest::RequestBuilder,
    ) -> Pin<Box<dyn Future<Output = Result<reqwest::RequestBuilder, ClawError>> + Send + 'a>> {
        Box::pin(async move {
            // For now, use basic auth headers.
            // Full SigV4 signing requires building the request first,
            // then signing it. This is a simplified placeholder that
            // sets up the credential headers. Production use should
            // use the aws-sigv4 crate for proper request signing.
            //
            // TODO: Implement full SigV4 signing with aws-sigv4 crate
            Err(ClawError::Auth(
                "Bedrock SigV4 signing not yet fully implemented. \
                 Use AWS SDK or proxy for Bedrock access."
                    .to_string(),
            ))
        })
    }

    fn override_url(&self, _base_url: &str, model: &str) -> Option<String> {
        Some(self.endpoint_url(model))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bedrock_endpoint_url() {
        let auth = BedrockAuth::new(
            "us-east-1".to_string(),
            "AKID".to_string(),
            "secret".to_string(),
            None,
        );
        let url = auth.endpoint_url("anthropic.claude-v2");
        assert!(url.contains("bedrock-runtime.us-east-1.amazonaws.com"));
        assert!(url.contains("anthropic.claude-v2"));
        assert!(url.contains("invoke-with-response-stream"));
    }
}

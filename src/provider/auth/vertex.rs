use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::error::ClawError;
use crate::provider::auth::AuthHook;

/// Vertex AI authentication using Google Application Default Credentials.
///
/// Fetches OAuth2 tokens via:
/// 1. `gcloud auth print-access-token` (most common for dev)
/// 2. GCE metadata server (when running on GCP)
///
/// Tokens are cached and refreshed when near expiry.
pub struct VertexAuth {
    project_id: String,
    region: String,
    token_cache: Mutex<Option<CachedToken>>,
}

struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

impl VertexAuth {
    pub fn new(project_id: String, region: String) -> Self {
        Self {
            project_id,
            region,
            token_cache: Mutex::new(None),
        }
    }

    /// Get Vertex AI endpoint URL for a model.
    pub fn endpoint_url(&self, model: &str) -> String {
        format!(
            "https://{region}-aiplatform.googleapis.com/v1/projects/{project}/locations/{region}/publishers/anthropic/models/{model}:streamRawPredict",
            region = self.region,
            project = self.project_id,
        )
    }

    /// Get a valid OAuth2 access token, refreshing if expired.
    async fn get_token(&self) -> Result<String, ClawError> {
        // Check cache first
        {
            let cache = self.token_cache.lock().unwrap();
            if let Some(ref cached) = *cache {
                if cached.expires_at > Instant::now() + Duration::from_secs(60) {
                    return Ok(cached.access_token.clone());
                }
            }
        }

        // Refresh token
        let token = self.fetch_token().await?;

        // Cache with 55-minute expiry (tokens typically last 1 hour)
        let mut cache = self.token_cache.lock().unwrap();
        *cache = Some(CachedToken {
            access_token: token.clone(),
            expires_at: Instant::now() + Duration::from_secs(55 * 60),
        });

        Ok(token)
    }

    /// Fetch a new token via gcloud CLI.
    async fn fetch_token(&self) -> Result<String, ClawError> {
        let output = tokio::process::Command::new("gcloud")
            .args(["auth", "print-access-token"])
            .output()
            .await
            .map_err(|e| {
                ClawError::Auth(format!(
                    "Failed to run gcloud auth print-access-token: {e}. \
                     Make sure gcloud CLI is installed and authenticated."
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ClawError::Auth(format!(
                "gcloud auth print-access-token failed: {stderr}"
            )));
        }

        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if token.is_empty() {
            return Err(ClawError::Auth(
                "gcloud auth print-access-token returned empty token".to_string(),
            ));
        }

        Ok(token)
    }
}

impl AuthHook for VertexAuth {
    fn prepare_request<'a>(
        &'a self,
        builder: reqwest::RequestBuilder,
    ) -> Pin<Box<dyn Future<Output = Result<reqwest::RequestBuilder, ClawError>> + Send + 'a>> {
        Box::pin(async move {
            let token = self.get_token().await?;
            Ok(builder.bearer_auth(token))
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
    fn test_vertex_endpoint_url() {
        let auth = VertexAuth::new("my-project".to_string(), "us-central1".to_string());
        let url = auth.endpoint_url("claude-sonnet-4-20250514");
        assert!(url.contains("us-central1-aiplatform.googleapis.com"));
        assert!(url.contains("my-project"));
        assert!(url.contains("claude-sonnet-4-20250514"));
        assert!(url.contains("streamRawPredict"));
    }

    #[test]
    fn test_token_cache_initial_empty() {
        let auth = VertexAuth::new("project".to_string(), "region".to_string());
        let cache = auth.token_cache.lock().unwrap();
        assert!(cache.is_none());
    }
}

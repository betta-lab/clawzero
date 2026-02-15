pub mod vertex;

#[cfg(feature = "bedrock")]
pub mod bedrock;

use std::future::Future;
use std::pin::Pin;

use crate::error::ClawError;

/// Hook for injecting authentication into HTTP requests.
/// Implemented by Vertex AI (OAuth2) and Bedrock (SigV4) auth providers.
pub trait AuthHook: Send + Sync {
    /// Modify a request builder to add authentication headers.
    fn prepare_request<'a>(
        &'a self,
        builder: reqwest::RequestBuilder,
    ) -> Pin<Box<dyn Future<Output = Result<reqwest::RequestBuilder, ClawError>> + Send + 'a>>;

    /// Override the base URL if needed (e.g. Vertex AI endpoint format).
    /// Returns None to use the default URL.
    fn override_url(&self, _base_url: &str, _model: &str) -> Option<String> {
        None
    }
}

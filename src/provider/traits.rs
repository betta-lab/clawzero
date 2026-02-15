use std::future::Future;
use std::pin::Pin;

use futures_util::Stream;

use crate::error::ClawError;
use crate::model::request::CompletionRequest;
use crate::model::response::StreamEvent;

/// A boxed, pinned, sendable stream of streaming events.
pub type EventStream = Pin<Box<dyn Stream<Item = Result<StreamEvent, ClawError>> + Send>>;

/// Core provider trait. Each protocol implementation (Anthropic, OpenAI)
/// implements this. Concrete providers are configured instances.
pub trait Provider: Send + Sync {
    /// Provider name (e.g., "anthropic", "openai", "openrouter")
    fn name(&self) -> &str;

    /// Send a completion request and return a streaming response.
    fn complete<'a>(
        &'a self,
        request: &'a CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<EventStream, ClawError>> + Send + 'a>>;
}

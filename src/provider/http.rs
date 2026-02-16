use std::time::Duration;

use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use futures_util::stream::Stream;
use reqwest::Response;

use crate::error::ClawError;

/// Build a shared HTTP client with connection pooling and timeouts.
pub fn build_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .connect_timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(4)
        .use_rustls_tls()
        .build()
        .expect("Failed to build HTTP client")
}

/// SSE event with event type and data payload.
#[derive(Debug)]
pub struct SseEvent {
    pub event_type: String,
    pub data: String,
}

/// Parse a streaming HTTP response body as SSE events.
pub fn parse_sse_stream(response: Response) -> impl Stream<Item = Result<SseEvent, ClawError>> {
    response
        .bytes_stream()
        .eventsource()
        .map(|result| match result {
            Ok(event) => Ok(SseEvent {
                event_type: event.event,
                data: event.data,
            }),
            Err(e) => Err(ClawError::Sse(e.to_string())),
        })
}

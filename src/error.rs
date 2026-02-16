#[derive(Debug, thiserror::Error)]
pub enum ClawError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Invalid JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("SSE stream error: {0}")]
    Sse(String),

    #[error("Provider error ({status}): {message}")]
    ProviderError { status: u16, message: String },

    #[error("Invalid model spec '{0}', expected 'provider/model' format")]
    InvalidModelSpec(String),

    #[error("Unknown provider: {0}")]
    UnknownProvider(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Tool execution failed: {0}")]
    ToolExecution(String),

    #[error("Rate limited, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("Overloaded: {0}")]
    Overloaded(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Auth error: {0}")]
    Auth(String),

    #[error("Gateway error: {0}")]
    Gateway(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),
}

# Provider System

## Two-protocol design

clawzero uses two protocol implementations to cover all major LLM providers:

| Protocol | Implementation | Providers |
|----------|---------------|-----------|
| Anthropic Messages API | `protocol/anthropic.rs` | Anthropic, Vertex AI, Bedrock |
| OpenAI Chat Completions API | `protocol/openai.rs` | OpenAI, OpenRouter, Ollama, vLLM |

This design means adding a new OpenAI-compatible provider (e.g., a new local inference server) requires only a config entry:

```toml
[providers.my-server]
protocol = "openai"
base_url = "http://localhost:8080"
api_key = ""
```

## Provider registry

The provider registry (`provider/registry.rs`) resolves model strings in `provider/model` format:

1. Parse `"anthropic/claude-sonnet-4-20250514"` → provider name `"anthropic"`, model `"claude-sonnet-4-20250514"`
2. Look up provider config from the `[providers]` table
3. Select protocol implementation based on `protocol` field
4. Wire up authentication if `auth` is set
5. Return a `Box<dyn Provider>` ready for streaming completion

## Provider trait

```rust
trait Provider {
    fn complete<'a>(
        &'a self,
        request: &'a CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<EventStream>> + Send + 'a>>;
}
```

The trait uses `Pin<Box<dyn Future>>` instead of `async fn` for dyn compatibility. The lifetime `'a` ties both `&self` and the request reference to ensure the stream can borrow from both.

## Authentication

Cloud providers use the `AuthHook` trait for authentication:

| Auth type | Method | Provider |
|-----------|--------|----------|
| `vertex` | OAuth2 via `gcloud auth print-access-token` | Vertex AI |
| `bedrock` | AWS SigV4 request signing | AWS Bedrock |

Auth hooks modify the HTTP request before sending (adding authorization headers, signing the request).

## SSE streaming

All providers use Server-Sent Events (SSE) for streaming:

```
reqwest response → bytes_stream → eventsource-stream → StreamEvent mapping
```

Each protocol implementation maps provider-specific SSE events to the common `StreamEvent` type (`TextDelta`, `ToolCall`, `Usage`, `Stop`).

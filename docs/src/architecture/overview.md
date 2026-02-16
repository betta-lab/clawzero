# Architecture Overview

## Module structure

```
src/
├── agent/              # Agent loop (Think → ToolCall → Observe)
│   ├── loop.rs         # Core loop with session saving
│   ├── factory.rs      # AgentFactory (shared Agent creation)
│   ├── context.rs      # Conversation context + compaction
│   ├── event.rs        # AgentEvent (UI notification)
│   ├── token.rs        # Token estimation (chars/4 heuristic)
│   └── compaction.rs   # DropOldest message compaction strategy
├── cli/                # CLI / REPL / TUI
│   ├── args.rs         # clap arg definitions
│   ├── repl.rs         # Plain text mode (interactive, one-shot, resume)
│   └── tui/            # ratatui-based inline TUI (Viewport::Inline)
│       ├── mod.rs      # run_tui_repl(), run_tui_oneshot()
│       ├── app.rs      # App state machine
│       ├── event.rs    # TuiEvent loop
│       ├── ui.rs       # Live viewport layout
│       ├── markdown.rs # Markdown → ratatui spans
│       └── widgets/    # Chat, status, input widgets
├── config/             # Configuration loading
│   ├── types.rs        # AppConfig, GatewayConfig, ProviderConfig
│   └── loader.rs       # TOML + env var merging
├── gateway/            # Multi-platform bot gateway
│   ├── session_map.rs  # ThreadKey → SessionID mapping
│   ├── event_handler.rs # AgentEvent → text with rate limiting
│   ├── slack/          # Slack (Socket Mode + Web API)
│   ├── discord/        # Discord (serenity EventHandler)
│   └── webui/          # Web UI (axum + WebSocket)
├── memory/             # Persistent memory (MEMORY.md)
│   └── store.rs        # Global + project memory
├── model/              # Provider-agnostic types
│   ├── message.rs      # Message, ContentBlock, Role
│   ├── request.rs      # CompletionRequest
│   ├── response.rs     # StreamEvent, StopReason, Usage
│   └── tool_schema.rs  # ToolDefinition
├── provider/           # LLM provider abstraction
│   ├── traits.rs       # Provider trait, EventStream
│   ├── http.rs         # Shared HTTP client + SSE parser
│   ├── registry.rs     # "provider/model" resolution
│   ├── auth/           # Vertex AI OAuth2, Bedrock SigV4
│   └── protocol/       # Anthropic + OpenAI implementations
├── session/            # JSONL session persistence
│   ├── types.rs        # SessionEntry, SessionMetadata
│   └── store.rs        # Session store + writer
├── tool/               # Tool system
│   ├── traits.rs       # Tool trait, ToolRegistry
│   ├── builtin/        # 6 built-in tools
│   └── plugin/         # Bash / HTTP plugin tools
└── error.rs            # ClawError
```

## Design principles

- **Two protocols cover all providers**: Anthropic Messages API and OpenAI Chat Completions API implementations handle every major provider. OpenRouter, Ollama, vLLM, etc. are OpenAI-compatible.
- **Config-driven**: Adding a new provider requires only a `[providers.xxx]` entry in TOML — no code changes.
- **Pin\<Box\<dyn Future\>\>**: Provider and Tool traits use `Pin<Box<dyn Future>>` instead of `async fn` for dyn compatibility (even in Rust 2024 edition, `async fn` in traits is not dyn-compatible).
- **Thin HTTP abstraction**: reqwest + eventsource-stream with full control. No heavy framework dependencies.
- **No Gateway trait**: Each platform is an async function, not a trait implementation. Shared via `AgentFactory` and `SessionMap` only.
- **Embedded UI**: Web UI is a single HTML file compiled into the binary via `include_str!`. Zero external assets, zero build step.

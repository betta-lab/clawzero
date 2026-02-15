# clawzero

Ultra-fast, stable AI agent CLI built in Rust. Inspired by [OpenClaw](https://github.com/openclaw/openclaw).

## Features

- **Streaming-first** — Real-time responses via SSE streaming
- **Multi-provider** — Switch between Anthropic / OpenAI / OpenRouter / Ollama and more via config alone
- **Extensible provider design** — Two protocol implementations (Anthropic Messages API + OpenAI Chat Completions API) cover all major providers. Adding a new provider requires zero code changes — just config
- **Agent loop** — Autonomous task execution via Think → ToolCall → Observe cycle
- **Built-in tools** — bash execution, file read/write/edit (4 tools)

## Installation

```bash
cargo install --path .
```

### Prerequisites

- Rust 1.80+ (edition 2024)
- [mise](https://mise.jdx.dev/) (recommended) — `mise install` sets up the toolchain

## Usage

```bash
# One-shot
clawzero "Write a fibonacci function in Rust"

# Specify model
clawzero --model openai/gpt-4o "Hello"

# Interactive REPL
clawzero chat

# Show config
clawzero config
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `OPENAI_API_KEY` | OpenAI API key |
| `CLAWZERO_MODEL` | Override default model (e.g. `openai/gpt-4o`) |

## Configuration

Configure via `~/.config/clawzero/config.toml` (global) or `clawzero.toml` (project-local, higher priority).

```toml
[defaults]
model = "anthropic/claude-sonnet-4-20250514"
max_tokens = 8192
max_turns = 25

[providers.anthropic]
protocol = "anthropic"
base_url = "https://api.anthropic.com"
api_key_env = "ANTHROPIC_API_KEY"

[providers.openai]
protocol = "openai"
base_url = "https://api.openai.com"
api_key_env = "OPENAI_API_KEY"

[providers.openrouter]
protocol = "openai"
base_url = "https://openrouter.ai/api"
api_key_env = "OPENROUTER_API_KEY"

[providers.ollama]
protocol = "openai"
base_url = "http://localhost:11434"
api_key = ""
```

## Architecture

```
src/
├── agent/          # Agent loop (Think → ToolCall → Observe)
│   ├── loop.rs     # Core loop
│   ├── context.rs  # Conversation context management
│   └── event.rs    # AgentEvent (UI notification)
├── cli/            # CLI / REPL
│   ├── args.rs     # clap arg definitions
│   └── repl.rs     # Interactive & one-shot execution
├── config/         # Configuration loading
│   ├── types.rs    # AppConfig, ProviderConfig
│   └── loader.rs   # TOML + env var merging
├── model/          # Provider-agnostic types
│   ├── message.rs  # Message, ContentBlock, Role
│   ├── request.rs  # CompletionRequest
│   ├── response.rs # StreamEvent, StopReason, Usage
│   └── tool_schema.rs # ToolDefinition
├── provider/       # LLM provider abstraction
│   ├── traits.rs   # Provider trait, EventStream
│   ├── http.rs     # Shared HTTP client + SSE parser
│   ├── registry.rs # "provider/model" resolution
│   └── protocol/
│       ├── anthropic.rs  # Anthropic Messages API
│       └── openai.rs     # OpenAI Chat Completions API
├── tool/           # Tool system
│   ├── traits.rs   # Tool trait, ToolRegistry
│   └── builtin/
│       ├── shell.rs      # Bash execution
│       ├── file_read.rs  # File read
│       ├── file_write.rs # File write
│       └── file_edit.rs  # File edit (search-and-replace)
├── error.rs        # ClawError
├── lib.rs
└── main.rs
```

### Design Principles

- **Two protocols cover all providers**: Anthropic Messages API and OpenAI Chat Completions API implementations handle every major provider. OpenRouter / Ollama / vLLM etc. are OpenAI-compatible.
- **Config-driven**: Adding a new provider is just a `[providers.xxx]` entry in TOML.
- **Pin<Box<dyn Future>>**: Provider and Tool traits use `Pin<Box<dyn Future>>` instead of `async fn` for dyn compatibility (even in Rust 2024 edition, `async fn` in traits is not dyn-compatible).
- **Thin HTTP abstraction**: reqwest + eventsource-stream with full control. No heavy framework dependencies.

## Roadmap

- [x] **Phase 1**: CLI agent core
  - Multi-provider LLM API (Anthropic / OpenAI compatible)
  - Agent loop
  - Built-in tools (bash, file_read, file_write, file_edit)
  - CLI / REPL
- [ ] **Phase 2**: Persistence & extensions
  - Session persistence (JSONL)
  - Memory system
  - Vertex AI / Bedrock auth
  - Plugin tool system
  - Context window management
- [ ] **Phase 3**: Messaging channel integration

## License

TBD

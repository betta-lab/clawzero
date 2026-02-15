<p align="center">
  <img src="docs/img/logo.png" alt="clawzero" width="480" />
</p>

# clawzero

Ultra-fast, stable AI agent CLI built in Rust. Inspired by [OpenClaw](https://github.com/openclaw/openclaw).

## Features

- **Streaming-first** — Real-time responses via SSE streaming
- **Multi-provider** — Switch between Anthropic / OpenAI / OpenRouter / Ollama and more via config alone
- **Extensible provider design** — Two protocol implementations (Anthropic Messages API + OpenAI Chat Completions API) cover all major providers. Adding a new provider requires zero code changes — just config
- **Agent loop** — Autonomous task execution via Think → ToolCall → Observe cycle
- **Built-in tools** — bash execution, file read/write/edit, memory read/write (6 tools)
- **Session persistence** — JSONL-based conversation history with resume support
- **Context window management** — Automatic token estimation and message compaction when nearing context limits
- **Memory system** — Persistent MEMORY.md files (global + project-local) injected into system prompt
- **Plugin tools** — Define custom bash/HTTP tools via TOML config
- **Cloud auth** — Vertex AI (OAuth2 via gcloud) and AWS Bedrock (SigV4) authentication

## Installation

### Pre-built binaries

Download from [GitHub Releases](https://github.com/betta-lab/clawzero/releases):

```bash
# Example: Linux x86_64
curl -LO https://github.com/betta-lab/clawzero/releases/latest/download/clawzero-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
tar xzf clawzero-*.tar.gz
sudo mv clawzero-*/clawzero /usr/local/bin/
```

### Build from source

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

# Resume a session
clawzero --resume <session-id> "Continue from where we left off"

# List sessions
clawzero sessions list

# Resume a session (subcommand)
clawzero sessions resume <session-id>

# Show config
clawzero config
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `OPENAI_API_KEY` | OpenAI API key |
| `CLAWZERO_MODEL` | Override default model (e.g. `openai/gpt-4o`) |
| `GCLOUD_PROJECT` | GCP project ID (for Vertex AI, if not in config) |
| `AWS_ACCESS_KEY_ID` | AWS credentials (for Bedrock) |
| `AWS_SECRET_ACCESS_KEY` | AWS credentials (for Bedrock) |
| `AWS_REGION` | AWS region (for Bedrock, default: `us-east-1`) |

## Configuration

Configure via `~/.config/clawzero/config.toml` (global) or `clawzero.toml` (project-local, higher priority).

```toml
[defaults]
model = "anthropic/claude-sonnet-4-20250514"
max_tokens = 8192
max_turns = 25
context_limit = 200000  # Token limit for context window management

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

# Vertex AI (uses gcloud CLI for OAuth2 tokens)
[providers.vertex-claude]
protocol = "anthropic"
base_url = "https://us-central1-aiplatform.googleapis.com"
auth = "vertex"
project_id = "my-gcp-project"
region = "us-central1"

# AWS Bedrock (requires --features bedrock)
# [providers.bedrock-claude]
# protocol = "anthropic"
# base_url = "https://bedrock-runtime.us-east-1.amazonaws.com"
# auth = "bedrock"
# region = "us-east-1"
```

### Plugin Tools

Define custom tools in your config file:

```toml
[[tools]]
name = "weather"
description = "Get current weather for a city"
type = "http"
url = "https://api.weather.example/v1/current?city={{city}}"
method = "GET"

[tools.input_schema.properties.city]
type = "string"
description = "City name"

[tools.input_schema]
required = ["city"]

[[tools]]
name = "deploy"
description = "Deploy to staging"
type = "bash"
command = "cd {{project_dir}} && make deploy-staging"

[tools.input_schema.properties.project_dir]
type = "string"
description = "Project directory path"
```

## Architecture

```
src/
├── agent/              # Agent loop (Think → ToolCall → Observe)
│   ├── loop.rs         # Core loop with session saving
│   ├── context.rs      # Conversation context + compaction
│   ├── event.rs        # AgentEvent (UI notification)
│   ├── token.rs        # Token estimation (chars/4 heuristic)
│   └── compaction.rs   # DropOldest message compaction strategy
├── cli/                # CLI / REPL
│   ├── args.rs         # clap arg definitions
│   └── repl.rs         # Interactive, one-shot, & resume execution
├── config/             # Configuration loading
│   ├── types.rs        # AppConfig, ProviderConfig, AuthType
│   └── loader.rs       # TOML + env var merging
├── memory/             # Persistent memory system
│   └── store.rs        # MEMORY.md read/write (global + project)
├── model/              # Provider-agnostic types
│   ├── message.rs      # Message, ContentBlock, Role
│   ├── request.rs      # CompletionRequest
│   ├── response.rs     # StreamEvent, StopReason, Usage
│   └── tool_schema.rs  # ToolDefinition
├── provider/           # LLM provider abstraction
│   ├── traits.rs       # Provider trait, EventStream
│   ├── http.rs         # Shared HTTP client + SSE parser
│   ├── registry.rs     # "provider/model" resolution + auth wiring
│   ├── auth/
│   │   ├── mod.rs      # AuthHook trait
│   │   ├── vertex.rs   # Vertex AI OAuth2 (gcloud CLI)
│   │   └── bedrock.rs  # AWS Bedrock SigV4 (feature-gated)
│   └── protocol/
│       ├── anthropic.rs  # Anthropic Messages API
│       └── openai.rs     # OpenAI Chat Completions API
├── session/            # Session persistence
│   ├── types.rs        # SessionEntry, SessionMetadata
│   └── store.rs        # JSONL session store + writer
├── tool/               # Tool system
│   ├── traits.rs       # Tool trait, ToolRegistry
│   ├── builtin/
│   │   ├── shell.rs         # Bash execution
│   │   ├── file_read.rs     # File read
│   │   ├── file_write.rs    # File write
│   │   ├── file_edit.rs     # File edit (search-and-replace)
│   │   ├── memory_read.rs   # Memory read tool
│   │   └── memory_write.rs  # Memory write tool
│   └── plugin/
│       ├── types.rs         # PluginToolConfig, template substitution
│       ├── bash_plugin.rs   # Bash command plugin
│       ├── http_plugin.rs   # HTTP endpoint plugin
│       └── loader.rs        # Plugin loader
├── error.rs            # ClawError
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
- [x] **Phase 2**: Persistence & extensions
  - Session persistence (JSONL with crash-safe append + flush)
  - Context window management (token estimation + DropOldest compaction)
  - Memory system (global + project-local MEMORY.md)
  - Plugin tool system (bash / HTTP with template substitution)
  - Vertex AI / Bedrock authentication (AuthHook trait)
- [ ] **Phase 3**: Messaging channel integration

## License

TBD

<p align="center">
  <img src="docs/img/logo.png" alt="clawzero" width="480" />
</p>

# clawzero

Ultra-fast, stable AI agent CLI built in Rust. Inspired by [OpenClaw](https://github.com/openclaw/openclaw).

## Features

- **Inline TUI** — Claude Code-style inline terminal UI that grows in-place (no full-screen takeover). Streaming text, tool cards, and status update live in the viewport; confirmed output scrolls into terminal history
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
- **Gateway** — Run as Slack / Discord bot with `clawzero gateway`. Session-per-thread, streaming message updates, rate-limited

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
# One-shot (inline TUI, shows result, exits automatically)
clawzero "Write a fibonacci function in Rust"

# Specify model
clawzero --model openai/gpt-4o "Hello"

# Interactive REPL (TUI mode — default when stdin is a TTY)
clawzero chat

# Plain text mode (no TUI)
clawzero --no-tui chat

# Resume a session
clawzero --resume <session-id> "Continue from where we left off"

# List sessions
clawzero sessions list

# Resume a session (subcommand)
clawzero sessions resume <session-id>

# Show config
clawzero config

# Start Slack gateway
clawzero gateway slack

# Start Discord gateway
clawzero gateway discord

# Start all configured gateways
clawzero gateway
```

### TUI Keybindings

| Key | Action |
|-----|--------|
| Enter | Send message |
| Ctrl+J | Insert newline |
| Ctrl+A / Home | Move cursor to beginning of line |
| Ctrl+E / End | Move cursor to end of line |
| Ctrl+K | Delete from cursor to end of line |
| Ctrl+W | Delete word before cursor |
| Ctrl+C | Quit |
| `/exit`, `/quit` | Quit |

The TUI is enabled by default when stdin is a TTY. Use `--no-tui` to fall back to plain text mode. Piped input (`echo "hello" | clawzero`) automatically uses plain text mode. Past output scrolls into terminal history and can be viewed with your terminal's scrollback.

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
| `SLACK_APP_TOKEN` | Slack Socket Mode app token (xapp-...) |
| `SLACK_BOT_TOKEN` | Slack bot token (xoxb-...) |
| `DISCORD_BOT_TOKEN` | Discord bot token |

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

### Gateway Configuration

```toml
# Slack — requires Socket Mode enabled in Slack app settings
[gateway.slack]
app_token_env = "SLACK_APP_TOKEN"   # xapp-... (Socket Mode)
bot_token_env = "SLACK_BOT_TOKEN"   # xoxb-... (Web API)

# Discord — requires Message Content Intent enabled
[gateway.discord]
bot_token_env = "DISCORD_BOT_TOKEN"
```

Tokens can also be set directly:

```toml
[gateway.slack]
app_token = "xapp-1-..."
bot_token = "xoxb-..."
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
CLI ─────────────────→ Agent (direct)

clawzero gateway
  ├─ SlackGateway ──→ AgentFactory + SessionMap ──→ Agent (per thread)
  └─ DiscordGateway ─→ AgentFactory + SessionMap ──→ Agent (per thread)
```

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
│   ├── args.rs         # clap arg definitions (--no-tui flag)
│   ├── repl.rs         # Plain text mode (interactive, one-shot, resume)
│   └── tui/            # ratatui-based inline TUI (Viewport::Inline)
│       ├── mod.rs      # run_tui_repl(), run_tui_oneshot()
│       ├── app.rs      # App state machine (mode, pending_inserts, input)
│       ├── event.rs    # TuiEvent loop (terminal + agent + tick)
│       ├── ui.rs       # Live viewport layout (streaming + status + input)
│       ├── markdown.rs # Markdown → ratatui spans conversion
│       └── widgets/    # chat helpers, status, input widgets
├── config/             # Configuration loading
│   ├── types.rs        # AppConfig, GatewayConfig, ProviderConfig
│   └── loader.rs       # TOML + env var merging
├── gateway/            # Multi-platform bot gateway
│   ├── session_map.rs  # ThreadKey → SessionID persistent mapping
│   ├── event_handler.rs # AgentEvent → text with rate limiting
│   ├── slack/          # Slack integration (feature: slack)
│   │   ├── socket.rs   # Socket Mode WebSocket connection
│   │   ├── api.rs      # Web API (post/update/react)
│   │   └── handler.rs  # SlackGateway orchestration
│   └── discord/        # Discord integration (feature: discord)
│       └── handler.rs  # serenity EventHandler + DiscordGateway
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
- **No Gateway trait**: Each platform is an async function, not a trait implementation. Shared via `AgentFactory` (Agent creation) and `SessionMap` (thread → session mapping) only.

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
- [x] **Phase 3**: Gateway — Multi-platform simultaneous connections
  - AgentFactory for shared Agent creation
  - Slack Socket Mode (WebSocket + Web API)
  - Discord bot (serenity EventHandler)
  - Session-per-thread with persistent SessionMap
  - Rate-limited streaming message updates (BotEventHandler)
  - `clawzero gateway` runs all configured gateways concurrently
- [x] **Phase 4**: Inline TUI
  - Claude Code-style inline TUI (Viewport::Inline + insert_before)
  - Confirmed output scrolls into terminal history; live viewport shows only active content
  - Streaming text display with Markdown rendering
  - Tool call cards with status indicators
  - Spinner animation during thinking/tool execution
  - Multi-line input (Ctrl+J for newline)
  - `--no-tui` flag for plain text fallback
  - Automatic plain text mode for non-TTY stdin (pipe support)

## License

TBD

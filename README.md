<p align="center">
  <img src="docs/img/logo.png" alt="clawzero" width="480" />
</p>

# clawzero

Ultra-fast, stable AI agent CLI built in Rust. Inspired by [OpenClaw](https://github.com/openclaw/openclaw).

**[Documentation](https://betta-lab.github.io/clawzero/)**

## Features

- **Inline TUI** — Claude Code-style inline terminal UI
- **Streaming-first** — Real-time SSE streaming responses
- **Multi-provider** — Anthropic / OpenAI / OpenRouter / Ollama / Vertex AI / Bedrock
- **Agent loop** — Think → ToolCall → Observe autonomous execution
- **Built-in tools** — bash, file read/write/edit, memory read/write
- **Session persistence** — JSONL-based conversation history with resume
- **Memory system** — Persistent MEMORY.md (global + project-local)
- **Plugin tools** — Custom bash/HTTP tools via TOML config
- **Gateway** — Slack / Discord / Web UI bot via `clawzero gateway`

## Quick Start

```bash
cargo install --path .
export ANTHROPIC_API_KEY="sk-ant-..."
clawzero "Hello, world!"
```

See the [documentation](https://betta-lab.github.io/clawzero/) for installation options, configuration, and usage details.

## License

TBD

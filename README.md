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

## Benchmark

Performance comparison vs Claude Code and OpenClaw (all using **Sonnet 4.5** model):

### 🚀 Startup Time (`--help`, no API calls)

```mermaid
%%{init: {'theme':'base', 'themeVariables': { 'primaryColor':'#10b981', 'primaryTextColor':'#fff', 'primaryBorderColor':'#059669', 'lineColor':'#6366f1', 'secondaryColor':'#ec4899', 'tertiaryColor':'#f59e0b'}}}%%
xychart-beta
    title "Startup Time (lower is better)"
    x-axis ["🦀 clawzero", "🤖 Claude Code", "🦞 OpenClaw"]
    y-axis "milliseconds" 0 --> 1500
    bar [2.16, 838, 1425]
```

<p align="center"><b>clawzero: 2ms</b> · Claude Code: 838ms (388x slower) · OpenClaw: 1,425ms (659x slower)</p>

### ⚡ E2E Response Time ("What is 1+1?")

```mermaid
%%{init: {'theme':'base', 'themeVariables': { 'primaryColor':'#10b981', 'primaryTextColor':'#fff', 'primaryBorderColor':'#059669', 'lineColor':'#6366f1', 'secondaryColor':'#ec4899', 'tertiaryColor':'#f59e0b'}}}%%
xychart-beta
    title "End-to-End Time (lower is better)"
    x-axis ["🦀 clawzero", "🤖 Claude Code", "🦞 OpenClaw"]
    y-axis "milliseconds" 0 --> 20000
    bar [2100, 4674, 18718]
```

<p align="center"><b>clawzero: 2.1s</b> · Claude Code: 4.7s (2.2x slower) · OpenClaw: 18.7s (8.9x slower)</p>

### ⏱️ Time to First Token (TTFT)

```mermaid
%%{init: {'theme':'base', 'themeVariables': { 'primaryColor':'#10b981', 'primaryTextColor':'#fff', 'primaryBorderColor':'#059669', 'lineColor':'#6366f1', 'secondaryColor':'#ec4899', 'tertiaryColor':'#f59e0b'}}}%%
xychart-beta
    title "Time to First Token (lower is better)"
    x-axis ["🦀 clawzero", "🤖 Claude Code", "🦞 OpenClaw"]
    y-axis "milliseconds" 0 --> 10000
    bar [1689, 3232, 9611]
```

<p align="center"><b>clawzero: 1.7s</b> · Claude Code: 3.2s (1.9x slower) · OpenClaw: 9.6s (5.7x slower)</p>

### 💾 Memory Usage (simple scenario)

```mermaid
%%{init: {'theme':'base', 'themeVariables': { 'primaryColor':'#10b981', 'primaryTextColor':'#fff', 'primaryBorderColor':'#059669', 'lineColor':'#6366f1', 'secondaryColor':'#ec4899', 'tertiaryColor':'#f59e0b'}}}%%
xychart-beta
    title "Peak Memory (lower is better)"
    x-axis ["🦀 clawzero", "🤖 Claude Code", "🦞 OpenClaw"]
    y-axis "megabytes" 0 --> 400
    bar [10.1, 242, 398]
```

<p align="center"><b>clawzero: 10 MB</b> · Claude Code: 242 MB (24x more) · OpenClaw: 398 MB (39x more)</p>

### Summary Table

| Scenario | Metric | clawzero | Claude Code | OpenClaw |
|---|---|---:|---:|---:|
| **startup** | E2E Time | **2.16 ms** | 838 ms (388x) | 1,425 ms (659x) |
| | Memory | **5.5 MB** | 217 MB (39x) | 256 MB (47x) |
| **simple** | E2E Time | **2,100 ms** | 4,674 ms (2.2x) | 18,718 ms (8.9x) |
| | TTFT | **1,689 ms** | 3,232 ms (1.9x) | 9,611 ms (5.7x) |
| | Memory | **10.1 MB** | 242 MB (24x) | 398 MB (39x) |
| **tool_use** | E2E Time | **4,423 ms** | 16,435 ms (3.7x) | 10,482 ms (2.4x) |
| | TTFT | **1,162 ms** | 12,529 ms (10.8x) | 8,774 ms (7.6x) |
| | Memory | **10.2 MB** | 245 MB (24x) | 399 MB (39x) |

> **Environment**: Docker (Ubuntu 24.04), all tools using `anthropic/claude-sonnet-4-5-20250929`
> **Tools**: `hyperfine` (E2E time), `/usr/bin/time -v` (memory), custom wrapper (TTFT)
> **Reproduce**: `docker compose -f bench/docker-compose.yml run bench`

## Quick Start

```bash
cargo install --path .
clawzero init          # Interactive setup — generates config with API keys
clawzero "Hello, world!"
```

See the [documentation](https://betta-lab.github.io/clawzero/) for installation options, configuration, and usage details.

## License

TBD

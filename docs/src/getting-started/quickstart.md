# Quick Start

## 1. Set up your API key

The easiest way to get started is with `clawzero init`:

```bash
clawzero init
```

This interactively prompts for your API keys and generates `~/.config/clawzero/config.toml`.

Alternatively, set your API key via environment variable:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

See [Environment Variables](../configuration/environment-variables.md) for other providers.

## 2. Run your first prompt

```bash
# One-shot mode — sends a prompt, shows the result, and exits
clawzero "Write a fibonacci function in Rust"
```

## 3. Start an interactive session

```bash
# Interactive REPL with TUI (default when stdin is a TTY)
clawzero chat
```

## 4. Use a different model

```bash
# Use OpenAI GPT-4o
export OPENAI_API_KEY="sk-..."
clawzero --model openai/gpt-4o "Hello"

# Use a local Ollama model
clawzero --model ollama/llama3 "Hello"
```

## 5. Explore further

- [CLI reference](../usage/cli.md) — All commands and flags
- [Configuration](../configuration/overview.md) — Config files, providers, and defaults
- [Tools](../tools/builtin-tools.md) — Built-in tools the agent can use

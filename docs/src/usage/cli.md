# CLI

## Commands

### Initialize configuration

Set up clawzero interactively — select providers, configure API keys, choose a default model, and optionally set up gateways. Generates `~/.config/clawzero/config.toml`:

```bash
clawzero init
```

The interactive wizard guides you through:

1. **Provider selection** — Choose from Anthropic, OpenAI, OpenRouter, Ollama (local), Vertex AI (Google Cloud), and Bedrock (AWS)
2. **Per-provider configuration** — API keys (masked input), base URLs, regions, and project IDs as needed
3. **Default model selection** — Pick from models available in your selected providers
4. **Gateway configuration** (optional) — Set up Slack, Discord, or WebUI bot gateways

If an API key is left empty, the config will reference the corresponding environment variable (e.g. `ANTHROPIC_API_KEY`) instead.

### One-shot mode

Send a prompt, get a response, and exit:

```bash
clawzero "Write a fibonacci function in Rust"
```

### Interactive chat

Start an interactive REPL session:

```bash
clawzero chat
```

### Model selection

Override the default model with `--model`:

```bash
clawzero --model openai/gpt-4o "Hello"
clawzero --model ollama/llama3 chat
```

The model format is `provider/model-name`. You can also set the default via the `CLAWZERO_MODEL` environment variable or in your [config file](../configuration/overview.md).

### Show config

Display the current configuration:

```bash
clawzero config
```

### Session management

```bash
# List all sessions
clawzero sessions list

# Resume a session (subcommand)
clawzero sessions resume <session-id>

# Resume a session (flag — works with any command)
clawzero --resume <session-id> "Continue from where we left off"
```

See [Session Management](session-management.md) for details.

### Gateway

Start platform bots:

```bash
# Start all configured gateways
clawzero gateway

# Start a specific platform
clawzero gateway slack
clawzero gateway discord
clawzero gateway webui
```

See [Gateway Overview](../gateway/overview.md) for details.

## Global flags

| Flag | Description |
|------|-------------|
| `--model <provider/model>` | Override default model |
| `--resume <session-id>` | Resume an existing session |
| `--no-tui` | Disable TUI, use plain text mode |
| `--version` | Show version |
| `--help` | Show help |

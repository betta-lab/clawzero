# Configuration Overview

clawzero uses TOML configuration files with a layered loading strategy.

## Config file locations

| File | Scope | Priority |
|------|-------|----------|
| `~/.config/clawzero/config.toml` | Global (user-wide) | Lower |
| `clawzero.toml` | Project-local (current directory) | Higher |

Project-local settings override global settings.

## Defaults

```toml
[defaults]
model = "anthropic/claude-sonnet-4-20250514"
max_tokens = 8192
max_turns = 25
context_limit = 200000
```

| Field | Default | Description |
|-------|---------|-------------|
| `model` | `anthropic/claude-sonnet-4-20250514` | Default model in `provider/model` format |
| `max_tokens` | `8192` | Maximum tokens per response |
| `max_turns` | `25` | Maximum agent loop turns |
| `context_limit` | `200000` | Token limit for context window management |

The `model` can also be overridden via the `CLAWZERO_MODEL` environment variable or the `--model` CLI flag.

## Full config example

```toml
[defaults]
model = "anthropic/claude-sonnet-4-20250514"
max_tokens = 8192
max_turns = 25
context_limit = 200000

[providers.anthropic]
protocol = "anthropic"
base_url = "https://api.anthropic.com"
api_key_env = "ANTHROPIC_API_KEY"

[providers.openai]
protocol = "openai"
base_url = "https://api.openai.com"
api_key_env = "OPENAI_API_KEY"

[gateway.slack]
app_token_env = "SLACK_APP_TOKEN"
bot_token_env = "SLACK_BOT_TOKEN"

[gateway.discord]
bot_token_env = "DISCORD_BOT_TOKEN"

[gateway.webui]
host = "127.0.0.1"
port = 3000
```

See [Providers](providers.md) for all provider configurations and [Environment Variables](environment-variables.md) for available env vars.

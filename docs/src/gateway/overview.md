# Gateway Overview

The gateway system allows clawzero to run as a bot on multiple platforms simultaneously. Each platform connection maintains session-per-thread isolation with persistent session mapping.

## Architecture

```
clawzero gateway
  ├─ SlackGateway ──→ AgentFactory + SessionMap ──→ Agent (per thread)
  ├─ DiscordGateway ─→ AgentFactory + SessionMap ──→ Agent (per thread)
  └─ WebuiGateway ──→ AgentFactory + SessionMap ──→ Agent (per connection)
```

## Starting gateways

```bash
# Start all configured gateways concurrently
clawzero gateway

# Start a specific platform
clawzero gateway slack
clawzero gateway discord
clawzero gateway webui
```

Only gateways with valid configuration (tokens set) will start. Missing tokens are reported as warnings.

## Shared components

- **AgentFactory** — Creates Agent instances with shared configuration (model, tools, system prompt)
- **SessionMap** — Persistent mapping from platform thread IDs to session IDs, stored as JSON (`~/.local/share/clawzero/session_map.json`)
- **BotEventHandler** — Converts AgentEvent stream to text with rate-limited message updates (avoids API throttling)

## Session-per-thread

Each platform thread (Slack thread, Discord channel) gets its own session. The session is created on first message and resumed on subsequent messages in the same thread. This provides persistent conversation context per thread.

## Configuration

Gateway tokens can be set via environment variables or directly in config:

```toml
[gateway.slack]
app_token_env = "SLACK_APP_TOKEN"
bot_token_env = "SLACK_BOT_TOKEN"

[gateway.discord]
bot_token_env = "DISCORD_BOT_TOKEN"

[gateway.webui]
host = "127.0.0.1"
port = 3000
```

See [Slack](slack.md), [Discord](discord.md), and [Web UI configuration](../configuration/overview.md) for platform-specific setup.

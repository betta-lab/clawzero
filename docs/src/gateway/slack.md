# Slack Gateway

clawzero connects to Slack via Socket Mode (WebSocket) and responds using the Web API.

## Prerequisites

1. Create a Slack app at [api.slack.com/apps](https://api.slack.com/apps)
2. Enable **Socket Mode** in the app settings
3. Generate an **App-Level Token** with `connections:write` scope → this is your `SLACK_APP_TOKEN` (`xapp-...`)
4. Under **OAuth & Permissions**, add the following bot scopes:
   - `chat:write`
   - `app_mentions:read`
   - `channels:history`
   - `groups:history`
   - `im:history`
   - `mpim:history`
5. Install the app to your workspace and copy the **Bot User OAuth Token** → this is your `SLACK_BOT_TOKEN` (`xoxb-...`)
6. Under **Event Subscriptions**, subscribe to:
   - `message.channels`
   - `message.groups`
   - `message.im`
   - `app_mention`

## Configuration

### Environment variables

```bash
export SLACK_APP_TOKEN="xapp-1-..."
export SLACK_BOT_TOKEN="xoxb-..."
```

### Config file

```toml
[gateway.slack]
app_token_env = "SLACK_APP_TOKEN"
bot_token_env = "SLACK_BOT_TOKEN"
```

Or with direct values:

```toml
[gateway.slack]
app_token = "xapp-1-..."
bot_token = "xoxb-..."
```

## Running

```bash
# Slack only
clawzero gateway slack

# All configured gateways
clawzero gateway
```

## Build

Slack support requires the `slack` feature flag:

```bash
cargo install --path . --features slack
```

## Behavior

- Each Slack thread gets its own agent session
- The bot responds in-thread with streaming message updates
- Message updates are rate-limited to avoid Slack API throttling
- Reactions (emoji) indicate processing status

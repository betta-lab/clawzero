# Discord Gateway

clawzero connects to Discord using the serenity library and responds to messages.

## Prerequisites

1. Create a Discord application at [discord.com/developers/applications](https://discord.com/developers/applications)
2. Under **Bot**, create a bot and copy the **Token** → this is your `DISCORD_BOT_TOKEN`
3. Enable **Message Content Intent** under **Bot → Privileged Gateway Intents**
4. Generate an invite URL under **OAuth2 → URL Generator**:
   - Scopes: `bot`
   - Bot Permissions: `Send Messages`, `Read Message History`
5. Invite the bot to your server using the generated URL

## Configuration

### Environment variables

```bash
export DISCORD_BOT_TOKEN="your-discord-bot-token"
```

### Config file

```toml
[gateway.discord]
bot_token_env = "DISCORD_BOT_TOKEN"
```

Or with a direct value:

```toml
[gateway.discord]
bot_token = "your-discord-bot-token"
```

## Running

```bash
# Discord only
clawzero gateway discord

# All configured gateways
clawzero gateway
```

## Build

Discord support requires the `discord` feature flag:

```bash
cargo install --path . --features discord
```

## Behavior

- Each Discord channel gets its own agent session
- The bot responds with streaming message edits
- Message updates are rate-limited to avoid Discord API throttling

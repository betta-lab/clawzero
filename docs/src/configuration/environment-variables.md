# Environment Variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `OPENAI_API_KEY` | OpenAI API key |
| `CLAWZERO_MODEL` | Override default model (e.g. `openai/gpt-4o`) |
| `GCLOUD_PROJECT` | GCP project ID (for Vertex AI, if not in config) |
| `AWS_ACCESS_KEY_ID` | AWS credentials (for Bedrock) |
| `AWS_SECRET_ACCESS_KEY` | AWS credentials (for Bedrock) |
| `AWS_REGION` | AWS region (for Bedrock, default: `us-east-1`) |
| `SLACK_APP_TOKEN` | Slack Socket Mode app token (`xapp-...`) |
| `SLACK_BOT_TOKEN` | Slack bot token (`xoxb-...`) |
| `DISCORD_BOT_TOKEN` | Discord bot token |

Environment variables can be referenced in config files via `api_key_env`, `app_token_env`, and `bot_token_env` fields. See [Providers](providers.md) and [Gateway](../gateway/overview.md) for details.

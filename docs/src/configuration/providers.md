# Providers

clawzero supports multiple LLM providers through two protocol implementations. Adding a new provider requires only a config entry — no code changes.

## Anthropic

### API Key

```toml
[providers.anthropic]
protocol = "anthropic"
base_url = "https://api.anthropic.com"
api_key_env = "ANTHROPIC_API_KEY"
```

### Claude Code setup-token

If you have a Claude Code setup-token (`sk-ant-oat01-...`), you can use it directly.

```toml
[providers.anthropic]
protocol = "anthropic"
base_url = "https://api.anthropic.com"
api_key = "sk-ant-oat01-..."
```

Run `clawzero init` and select "Claude Code setup-token" when prompted for the Anthropic authentication method.

## OpenAI

```toml
[providers.openai]
protocol = "openai"
base_url = "https://api.openai.com"
api_key_env = "OPENAI_API_KEY"
```

## OpenRouter

```toml
[providers.openrouter]
protocol = "openai"
base_url = "https://openrouter.ai/api"
api_key_env = "OPENROUTER_API_KEY"
```

## Ollama (local)

```toml
[providers.ollama]
protocol = "openai"
base_url = "http://localhost:11434"
api_key = ""
```

No API key required for local Ollama.

## Vertex AI

Uses `gcloud` CLI for OAuth2 token authentication:

```toml
[providers.vertex-claude]
protocol = "anthropic"
base_url = "https://us-central1-aiplatform.googleapis.com"
auth = "vertex"
project_id = "my-gcp-project"
region = "us-central1"
```

Requires `gcloud` CLI to be installed and authenticated (`gcloud auth print-access-token`). Set `GCLOUD_PROJECT` env var or configure `project_id` in config.

## AWS Bedrock

Requires the `bedrock` feature flag:

```bash
cargo install --path . --features bedrock
```

```toml
[providers.bedrock-claude]
protocol = "anthropic"
base_url = "https://bedrock-runtime.us-east-1.amazonaws.com"
auth = "bedrock"
region = "us-east-1"
```

Requires AWS credentials (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`) and optionally `AWS_REGION`.

## Provider config fields

| Field | Type | Description |
|-------|------|-------------|
| `protocol` | `"anthropic"` or `"openai"` | API protocol to use |
| `base_url` | string | API base URL |
| `api_key` | string | API key (direct value) |
| `api_key_env` | string | Env var name for API key |
| `auth` | `"vertex"` or `"bedrock"` | Cloud authentication method |
| `project_id` | string | GCP project ID (Vertex AI) |
| `region` | string | Cloud region (Vertex AI / Bedrock) |
| `extra_headers` | table | Additional HTTP headers |
| `models` | array | Restrict available models |

## Model format

Models are specified in `provider/model` format:

```bash
clawzero --model anthropic/claude-opus-4-6 "Hello"
clawzero --model openai/gpt-4o "Hello"
clawzero --model ollama/llama3 "Hello"
clawzero --model openrouter/meta-llama/llama-3-70b "Hello"
```

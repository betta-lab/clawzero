# Contributing

Thank you for your interest in contributing to clawzero! This guide covers the development workflow and conventions.

## Prerequisites

- **Rust** (latest stable)
- **[mise](https://mise.jdx.dev/)** — manages Rust and mdBook versions
- **mdBook** (latest, installed via mise)

## Setup

```bash
git clone https://github.com/betta-lab/clawzero.git
cd clawzero
cargo build
```

mise automatically provides the correct tool versions.

## Test-Driven Development

This project follows **TDD (Test-Driven Development)**:

1. **Red** — Write a failing test first
2. **Green** — Write the minimum code to make the test pass
3. **Refactor** — Clean up while keeping tests green

```bash
# Unit tests
cargo test --lib --bins

# Full test suite (includes e2e, requires API keys)
cargo test
```

## Code Quality

All PRs must pass CI checks:

```bash
cargo fmt --check            # Formatting
cargo clippy -- -D warnings  # Linting (warnings = errors)
cargo test --lib --bins      # Tests
cargo build --release        # Release build
mdbook build docs            # Documentation build
```

## Documentation

**README.md** and **docs/** are the Single Source of Truth for documentation. Both must be updated before committing any user-facing changes.

- All content in `docs/` must be written in **English**
- Preview locally: `mdbook serve docs`

## Project Structure

```
src/
├── main.rs          # CLI entry point
├── agent/           # Core agent loop & session management
├── cli/             # CLI, REPL, TUI interface
├── config/          # Configuration (TOML + env)
├── provider/        # Multi-provider abstraction
├── tool/            # Tool system (builtin + plugins)
├── gateway/         # Slack / Discord / Web UI bots
├── memory/          # Persistent memory system
├── model/           # Provider-agnostic types
├── session/         # JSONL session persistence
└── error.rs         # Error handling
```

## Pull Request Process

1. Fork the repository and create a feature branch from `main`
2. Write tests first (TDD), then implement
3. Run all checks: `cargo fmt --check && cargo clippy -- -D warnings && cargo test --lib --bins && mdbook build docs`
4. Update documentation if changes affect user-facing behavior
5. Submit a PR against `main`

## Feature Flags

| Flag | Description |
|---|---|
| `slack` | Slack gateway (tokio-tungstenite) |
| `discord` | Discord gateway (serenity 0.12) |
| `bedrock` | AWS Bedrock provider |

```bash
cargo build --features slack,discord
```

## Extending clawzero

### Adding a Provider

Providers use a protocol-based abstraction:

- `AnthropicProtocol` — for Anthropic-compatible APIs
- `OpenAiProtocol` — for OpenAI-compatible APIs

Register new providers in the config-driven registry with `"provider/model"` format (e.g., `anthropic/claude-opus-4-6`).

### Adding a Tool

Built-in tools implement the `Tool` trait (`src/tool/`). The trait uses `Pin<Box<dyn Future>>` for dyn compatibility.

For simpler integrations, consider [Plugin Tools](../tools/plugin-tools.md) — custom bash/HTTP tools via TOML config, no Rust code required.

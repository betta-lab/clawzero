# Contributing to clawzero

Thank you for your interest in contributing to clawzero! This guide will help you get started.

## Prerequisites

- **Rust** (latest stable)
- **[mise](https://mise.jdx.dev/)** — manages Rust and mdBook versions automatically
- **mdBook** (latest, installed via mise)

## Setup

```bash
git clone https://github.com/betta-lab/clawzero.git
cd clawzero
cargo build
```

mise will automatically provide the correct versions of Rust and mdBook.

## Development Workflow

### Test-Driven Development

This project follows **TDD (Test-Driven Development)**. When implementing a feature or fixing a bug:

1. **Red** — Write a failing test first
2. **Green** — Write the minimum code to make the test pass
3. **Refactor** — Clean up the code while keeping tests green

### Running Tests

```bash
# Unit tests
cargo test --lib --bins

# Full test suite (includes e2e tests, requires API keys)
cargo test
```

### Code Quality

All PRs must pass the CI checks:

```bash
cargo fmt --check      # Code formatting
cargo clippy -- -D warnings  # Linting (warnings are errors)
cargo test --lib --bins      # Tests
cargo build --release        # Release build
mdbook build docs            # Documentation build
```

Format and lint your code before committing:

```bash
cargo fmt
cargo clippy -- -D warnings
```

## Documentation

**README.md** and **docs/** (mdBook) are the Single Source of Truth. Both must be updated before committing any user-facing changes.

- `docs/` content must be written in **English**
- Run `mdbook serve docs` to preview documentation locally
- Published at: https://betta-lab.github.io/clawzero/

## Project Structure

```
src/
├── main.rs          # CLI entry point
├── lib.rs           # Library entry
├── agent/           # Core agent loop & session management
├── cli/             # CLI, REPL, TUI interface
├── config/          # Configuration loading (TOML + env)
├── provider/        # Multi-provider abstraction
├── tool/            # Tool system (builtin + plugins)
├── gateway/         # Slack / Discord / Web UI bots
├── memory/          # Persistent memory system
├── model/           # Provider-agnostic types
├── session/         # JSONL session persistence
└── error.rs         # Error handling

tests/
└── e2e.rs           # End-to-end tests (assert_cmd)

docs/                # mdBook documentation
bench/               # Docker-based benchmark suite
```

## Pull Request Process

1. Fork the repository and create a feature branch from `main`
2. Write tests first (TDD), then implement your changes
3. Ensure all checks pass: `cargo fmt --check && cargo clippy -- -D warnings && cargo test --lib --bins && mdbook build docs`
4. Update documentation (README.md and docs/) if your changes affect user-facing behavior
5. Submit a PR against `main`

## Feature Flags

clawzero uses feature flags for optional integrations:

| Flag | Description |
|---|---|
| `slack` | Slack gateway (tokio-tungstenite) |
| `discord` | Discord gateway (serenity 0.12) |
| `bedrock` | AWS Bedrock provider |

Build with specific features:

```bash
cargo build --features slack,discord
```

## Adding a New Provider

Providers use a protocol-based abstraction. Two protocols cover all providers:

- `AnthropicProtocol` — for Anthropic-compatible APIs
- `OpenAiProtocol` — for OpenAI-compatible APIs

To add a new provider, register it in the config-driven provider registry with `"provider/model"` format (e.g., `anthropic/claude-opus-4-6`).

## Adding a New Tool

Built-in tools implement the `Tool` trait in `src/tool/`. The trait uses `Pin<Box<dyn Future>>` for dyn compatibility.

For simpler integrations, consider using [Plugin Tools](https://betta-lab.github.io/clawzero/tools/plugin-tools.html) — custom bash/HTTP tools defined via TOML config, requiring no Rust code.

## Questions?

Open an issue on [GitHub](https://github.com/betta-lab/clawzero/issues) for questions or discussions.

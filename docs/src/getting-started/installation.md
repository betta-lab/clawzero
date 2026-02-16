# Installation

## Install script (recommended)

Download and install the latest pre-built binary:

```bash
curl -fsSL https://raw.githubusercontent.com/betta-lab/clawzero/main/install.sh | sh
```

By default, the binary is installed to `~/.local/bin`. You can change this with the `INSTALL_DIR` environment variable:

```bash
INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/betta-lab/clawzero/main/install.sh | sh
```

Supported targets:
- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

## Manual download

Download from [GitHub Releases](https://github.com/betta-lab/clawzero/releases):

```bash
# Example: Linux x86_64
curl -LO https://github.com/betta-lab/clawzero/releases/latest/download/clawzero-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
tar xzf clawzero-*.tar.gz
sudo mv clawzero-*/clawzero /usr/local/bin/
```

## Build from source

```bash
cargo install --path .
```

### Optional feature flags

```bash
# Enable Slack gateway
cargo install --path . --features slack

# Enable Discord gateway
cargo install --path . --features discord

# Enable AWS Bedrock authentication
cargo install --path . --features bedrock

# Enable all features
cargo install --path . --features "slack,discord,bedrock"
```

## Prerequisites

- Rust 1.80+ (edition 2024)
- [mise](https://mise.jdx.dev/) (recommended) — `mise install` sets up the toolchain

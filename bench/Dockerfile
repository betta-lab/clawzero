# Stage 1: Build clawzero
FROM rust:latest AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release

# Stage 2: Runtime environment with benchmark tools
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    gnupg \
    time \
    moreutils \
    python3 \
    gawk \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js 22
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

# Install hyperfine
RUN curl -fsSL https://github.com/sharkdp/hyperfine/releases/download/v1.19.0/hyperfine_1.19.0_amd64.deb \
    -o /tmp/hyperfine.deb \
    && dpkg -i /tmp/hyperfine.deb \
    && rm /tmp/hyperfine.deb

# Install Claude Code and OpenClaw globally
RUN npm install -g @anthropic-ai/claude-code 2>/dev/null || true
RUN npm install -g openclaw 2>/dev/null || true

# Configure OpenClaw to use Sonnet 4.5 (same as clawzero and Claude Code)
RUN mkdir -p /root/.openclaw && echo '{\n\
  "agents": {\n\
    "defaults": {\n\
      "model": {\n\
        "primary": "anthropic/claude-sonnet-4-5-20250929"\n\
      }\n\
    }\n\
  }\n\
}' > /root/.openclaw/openclaw.json

# Copy clawzero binary from builder
COPY --from=builder /build/target/release/clawzero /usr/local/bin/clawzero

# Copy bench scripts
COPY bench/ /bench/

WORKDIR /bench

ENTRYPOINT ["./run.sh"]

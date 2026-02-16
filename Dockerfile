FROM rust:1.93-slim-bullseye AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies by building them first
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && echo '' > src/lib.rs \
    && cargo build --release \
    && rm -rf src target/release/.fingerprint/clawzero-*

# Build the actual binary
COPY src/ src/
RUN cargo build --release

# ---

FROM debian:bullseye

RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    curl \
    wget \
    jq \
    ripgrep \
    tree \
    less \
    openssh-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY --from=builder /app/target/release/clawzero /usr/local/bin/clawzero

ENTRYPOINT ["clawzero"]

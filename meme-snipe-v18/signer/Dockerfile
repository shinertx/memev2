# syntax=docker/dockerfile:1.4
FROM rust:1.75-slim AS builder

ARG BUILDKIT_INLINE_CACHE=1

RUN --mount=type=cache,target=/var/cache/apt \
    apt-get update && \
    apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    clang \
    ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy workspace configs for signer only
COPY signer-build-Cargo.toml Cargo.toml
COPY Cargo.lock ./
COPY shared-models/Cargo.toml shared-models/
COPY signer/Cargo.toml signer/

# Create dummy files for signer and shared-models only
RUN mkdir -p signer/src shared-models/src && \
    echo "fn main() {}" > signer/src/main.rs && \
    echo "pub fn dummy() {}" > shared-models/src/lib.rs

# Build dependencies
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release --package signer

# Copy actual source
RUN rm -rf signer/src shared-models/src
COPY shared-models/src shared-models/src
COPY signer/src signer/src

# Build final binary
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release --package signer && \
    cp target/release/signer /signer

# Runtime - minimal debian with necessary tools
FROM debian:12-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    wget \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /signer /app/signer

WORKDIR /app
EXPOSE 8989

ENTRYPOINT ["/app/signer"]

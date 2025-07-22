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

# Copy workspace configs
COPY meta-allocator-build-Cargo.toml ./Cargo.toml
COPY meta_allocator/Cargo.toml meta_allocator/
COPY shared-models/Cargo.toml shared-models/

# Create dummy files
RUN for member in meta_allocator shared-models; do \
        mkdir -p $member/src && \
        if [ "$member" = "shared-models" ]; then \
            echo "pub fn dummy() {}" > $member/src/lib.rs; \
        else \
            echo "fn main() {}" > $member/src/main.rs; \
        fi; \
    done

# Build dependencies
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release --workspace

# Copy actual source
RUN rm -rf meta_allocator/src shared-models/src
COPY shared-models/src shared-models/src
COPY meta_allocator/src meta_allocator/src

# Build final binary
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release --bin meta_allocator && \
    cp target/release/meta_allocator /meta_allocator

# Runtime - distroless
FROM gcr.io/distroless/cc-debian12

COPY --from=builder /meta_allocator /app/meta_allocator

WORKDIR /app

ENTRYPOINT ["/app/meta_allocator"]

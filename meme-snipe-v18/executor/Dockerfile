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
COPY Cargo.toml Cargo.lock ./
COPY executor/Cargo.toml executor/
COPY shared-models/Cargo.toml shared-models/
COPY signer/Cargo.toml signer/
COPY meta_allocator/Cargo.toml meta_allocator/
COPY position_manager/Cargo.toml position_manager/

# Create dummy files
RUN for member in executor shared-models signer meta_allocator position_manager; do \
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
RUN rm -rf executor/src shared-models/src
COPY shared-models/src shared-models/src
COPY executor/src executor/src

# Build final binary
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release --bin executor && \
    cp target/release/executor /executor

# Runtime - distroless for security
FROM gcr.io/distroless/cc-debian12

COPY --from=builder /executor /app/executor

WORKDIR /app
EXPOSE 9184

ENTRYPOINT ["/app/executor"]

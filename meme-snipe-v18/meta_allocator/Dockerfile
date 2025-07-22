# Stage 1: Build the application
FROM rust:1.88-slim AS builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends build-essential pkg-config libssl-dev clang \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy dependency manifests
COPY ./Cargo.lock .
COPY ./meta_allocator/Cargo.toml meta_allocator/Cargo.toml
COPY ./shared-models/Cargo.toml shared-models/Cargo.toml

# Create dummy source files for dependency build
RUN mkdir -p meta_allocator/src shared-models/src && \
    echo 'fn main(){}' > meta_allocator/src/main.rs && \
    echo 'pub fn lib(){}' > shared-models/src/lib.rs

RUN cargo build --release --workspace

# Clean up dummy files
RUN rm meta_allocator/src/*.rs shared-models/src/*.rs

# Copy actual source code
COPY ./meta_allocator/src ./meta_allocator/src
COPY ./shared-models/src ./shared-models/src

# Build final binary
RUN cargo build --release --bin meta_allocator

# Stage 2: Create the final, minimal image
FROM debian:bookworm-slim@sha256:46d9c8b8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

RUN groupadd --system app && useradd --system --gid app app
USER app

WORKDIR /app

COPY --from=builder /app/target/release/meta_allocator .

CMD ["./meta_allocator"]

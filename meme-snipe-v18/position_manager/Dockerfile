# MemeSnipe v18 - Position Manager Dockerfile
# Stage 1: Build the application
FROM rust:1.75-slim@sha256:e0b2b5c7e2fb0ad3f7ac44c8ed06d4694931a7201c13d8031a0e0513e9a7e939 AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends build-essential pkg-config libssl-dev clang

WORKDIR /app

# Copy dependency manifests from the workspace root
COPY ./Cargo.lock ./Cargo.lock
COPY ./position_manager/Cargo.toml ./position_manager/Cargo.toml
COPY ./shared-models/Cargo.toml ./shared-models/Cargo.toml

# Create dummy source files for dependency build
RUN mkdir -p position_manager/src && echo "fn main() {}" > position_manager/src/main.rs
RUN mkdir -p shared-models/src && echo "pub fn lib() {}" > shared-models/src/lib.rs
RUN cargo build --release --workspace

# Clean up dummy files
RUN rm position_manager/src/*.rs shared-models/src/*.rs

# Now, copy the actual source code
COPY ./position_manager/src ./position_manager/src
COPY ./shared-models/src ./shared-models/src

# Build the final binary
RUN cargo build --release --bin position_manager

# Stage 2: Create the final, minimal image
FROM debian:bookworm-slim

# Install only necessary runtime libraries
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Create a non-root user for security
RUN groupadd --system app && useradd --system --gid app app
USER app

WORKDIR /app

# Copy the built binary from the builder stage
COPY --from=builder /app/target/release/position_manager .

# Set LIBCLANG_PATH for runtime environment
ENV LIBCLANG_PATH=/usr/lib/llvm-14/lib

CMD ["./position_manager"]

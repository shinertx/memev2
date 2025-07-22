# Stage 1: Build the application
FROM rust:1.75-slim@sha256:e0b2b5c7e2fb0ad3f7ac44c8ed06d4694931a7201c13d8031a0e0513e9a7e939 AS builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends build-essential pkg-config libssl-dev clang

WORKDIR /app

# Copy dependency manifests
COPY ./Cargo.lock .
COPY ./signer/Cargo.toml signer/Cargo.toml
COPY ./shared-models/Cargo.toml shared-models/Cargo.toml

# Create dummy source files for dependency build
RUN mkdir -p signer/src shared-models/src && \
    echo 'fn main(){}' > signer/src/main.rs && \
    echo 'pub fn lib(){}' > shared-models/src/lib.rs

RUN cargo build --release --workspace

# Clean up dummy files
RUN rm signer/src/*.rs shared-models/src/*.rs

# Copy actual source code
COPY ./signer/src ./signer/src
COPY ./shared-models/src ./shared-models/src

# Build final binary
RUN cargo build --release --bin signer


# --- Stage 2: Create the final, minimal image ------------------------------
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

# Create a non-root user for security
RUN groupadd --system app && useradd --system --gid app app
USER app

WORKDIR /app

COPY --from=builder /app/target/release/signer .

CMD ["./signer"]

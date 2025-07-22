# syntax=docker/dockerfile:1.4
FROM python:3.11-slim AS builder

# Install build dependencies
RUN --mount=type=cache,target=/var/cache/apt \
    apt-get update && \
    apt-get install -y --no-install-recommends \
    gcc python3-dev && \
    rm -rf /var/lib/apt/lists/*

# Install Python dependencies
WORKDIR /build
COPY data_consumers/requirements.txt .
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install --no-cache-dir --upgrade pip && \
    pip install --no-cache-dir --target=/install -r requirements.txt

# Runtime - minimal Python
FROM python:3.11-slim

# Create non-root user
RUN groupadd -g 1000 appuser && \
    useradd -r -u 1000 -g appuser appuser

# Copy Python packages
COPY --from=builder /install /usr/local/lib/python3.11/site-packages

WORKDIR /app

# Create healthcheck script with proper ownership
RUN echo '#!/bin/sh\npgrep -f "$1" > /dev/null' > /app/healthcheck.sh && \
    chmod +x /app/healthcheck.sh && \
    chown appuser:appuser /app/healthcheck.sh

USER appuser

# Copy application code
COPY --chown=appuser:appuser data_consumers/*.py ./

# CMD specified in docker-compose.yml

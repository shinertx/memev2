# syntax=docker/dockerfile:1.4
FROM python:3.11-slim AS builder

WORKDIR /build
COPY dashboard/requirements.txt .
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install --no-cache-dir --upgrade pip && \
    pip install --no-cache-dir --target=/install -r requirements.txt

# Runtime
FROM python:3.11-slim

# Install runtime dependencies
RUN --mount=type=cache,target=/var/cache/apt \
    apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates && \
    rm -rf /var/lib/apt/lists/*

RUN groupadd -g 1000 appuser && \
    useradd -r -u 1000 -g appuser appuser

COPY --from=builder /install /usr/local/lib/python3.11/site-packages

WORKDIR /app

# Create instance directory with proper permissions before switching user
RUN mkdir -p /app/instance && \
    chown -R appuser:appuser /app

USER appuser

# Copy application
COPY --chown=appuser:appuser dashboard/app.py .
COPY --chown=appuser:appuser dashboard/templates ./templates

EXPOSE 5000

CMD ["python", "-m", "flask", "run", "--host=0.0.0.0"]

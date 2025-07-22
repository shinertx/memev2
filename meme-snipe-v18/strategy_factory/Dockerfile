# syntax=docker/dockerfile:1.4
FROM python:3.11-slim AS builder

WORKDIR /build
COPY strategy_factory/requirements.txt .
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install --no-cache-dir --upgrade pip && \
    pip install --no-cache-dir --target=/install -r requirements.txt

# Runtime
FROM python:3.11-slim

RUN groupadd -g 1000 appuser && \
    useradd -r -u 1000 -g appuser appuser

COPY --from=builder /install /usr/local/lib/python3.11/site-packages

WORKDIR /app
USER appuser

COPY --chown=appuser:appuser strategy_factory/factory.py .

CMD ["python", "-u", "factory.py"]

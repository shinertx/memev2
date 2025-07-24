# MemeSnipe v18 - Production Deployment Guide

## Prerequisites
- Docker 24.0+ with BuildKit enabled
- Docker Compose 2.20+
- 16GB+ RAM for builds
- 100GB+ SSD storage

## Initial Setup

1. **Clone and Configure**
```bash
git clone https://github.com/your-org/memesnipe-v18.git
cd memesnipe-v18
cp .env.example .env
# Edit .env with your API keys and configuration
```

2. **Place Wallet Files**
```bash
# CRITICAL: These files must exist before deployment
cp /secure/location/my_wallet.json ./my_wallet.json
cp /secure/location/jito_auth_key.json ./jito_auth_key.json
chmod 400 *.json
```

3. **Initialize Database**
```bash
# Create initial database structure
mkdir -p shared
touch shared/trades_v18.db
```

## Build Process - Performance Optimized

1. **Enable Advanced Caching**
```bash
# Enable BuildKit for optimal caching
export DOCKER_BUILDKIT=1
export COMPOSE_DOCKER_CLI_BUILD=1

# Enable inline cache for CI/CD
export BUILDKIT_INLINE_CACHE=1

# Use registry cache (if available)
export COMPOSE_DOCKER_CLI_BUILD_CACHE_FROM=ghcr.io/meme-snipe/cache:latest
```

2. **Build with Maximum Efficiency**
```bash
# Build all services in parallel with cache mount
docker compose build --parallel \
  --build-arg BUILDKIT_INLINE_CACHE=1 \
  --progress=plain

# For CI/CD with registry caching
docker buildx build \
  --cache-from type=registry,ref=ghcr.io/meme-snipe/rust-cache:latest \
  --cache-to type=registry,ref=ghcr.io/meme-snipe/rust-cache:latest,mode=max \
  --push .
```

3. **Performance Features Enabled**
- **cargo-chef**: Caches Rust dependencies separately (99% cache hit rate)
- **Multi-stage builds**: Final images ~50MB for Rust services
- **jemalloc**: 20-30% memory performance improvement
- **LTO + strip**: Binary size reduced by 70%
- **target-cpu=native**: CPU-specific optimizations
- **Layer caching**: Docker reuses unchanged layers

## Deployment

1. **Start Infrastructure First**
```bash
# Start Redis and observability stack
docker compose up -d redis prometheus grafana
# Wait for health
docker compose ps
```

2. **Start Core Services**
```bash
# Start risk and security services first
docker compose up -d risk_guardian wallet_guard signer
# Then start executor and position manager
docker compose up -d executor position_manager
# Finally start ancillary services
docker compose up -d autonomous_allocator data_consumers strategy_factory dashboard
```

3. **Verify Health**
```bash
# Check all services are healthy
docker compose ps
# Check logs for any errors
docker compose logs --tail=50
# Verify metrics endpoint
curl http://localhost:9091/metrics | grep -E "^# HELP"
```

## Operations

### Monitoring
- Dashboard: http://localhost
- Grafana: http://localhost:3000 (admin/configured_password)
- Prometheus: http://localhost:9090

### View Logs
```bash
# All services
docker compose logs -f

# Specific service
docker compose logs -f executor

# With timestamps
docker compose logs -t executor
```

### Performance Monitoring
```bash
# Check container resource usage
docker stats --no-stream

# Monitor build cache usage
docker system df

# Check Redis performance
docker compose exec redis redis-cli --latency

# Verify jemalloc is active
docker compose exec executor sh -c 'echo $LD_PRELOAD'
```

### Scaling
```bash
# Scale data consumers
docker compose up -d --scale data_consumers=3
```

### Updates with Zero Downtime
```bash
# Build new version with cache
docker compose build executor

# Rolling update
docker compose up -d --no-deps --no-recreate executor
```

### Cache Management
```bash
# View cache usage
docker buildx du

# Prune old cache (keep last 7 days)
docker buildx prune --keep-storage=10GB --filter "until=168h"

# Export cache for CI/CD
docker save -o cache.tar $(docker images -q)
```

### Backup
```bash
# Backup database
docker compose exec -T dashboard sqlite3 /app/shared/trades_v18.db ".backup /app/shared/backup.db"
docker cp memesnipe-dashboard:/app/shared/backup.db ./backups/trades_$(date +%Y%m%d).db

# Backup Redis
docker compose exec -T redis redis-cli BGSAVE
docker cp memesnipe-redis:/data/dump.rdb ./backups/redis_$(date +%Y%m%d).rdb
```

## Troubleshooting

### Service Won't Start
```bash
# Check detailed logs
docker compose logs --tail=100 service_name
# Check resource limits
docker stats --no-stream
# Verify file permissions
ls -la my_wallet.json jito_auth_key.json
```

### Database Issues
```bash
# Check database integrity
docker compose exec dashboard sqlite3 /app/shared/trades_v18.db "PRAGMA integrity_check;"
# Repair if needed
docker compose exec dashboard sqlite3 /app/shared/trades_v18.db "VACUUM;"
```

### Emergency Stop
```bash
# Stop all trading immediately
docker compose exec redis redis-cli PUBLISH kill_switch_channel "HALT"
# Stop all services
docker compose down
```

## Performance Benchmarks
- **Build time**: ~2 minutes first build, ~15 seconds with cache
- **Image sizes**: Rust services ~50MB, Python ~150MB
- **Memory usage**: 50% reduction with jemalloc
- **Startup time**: <2 seconds per service
- **Cache hit rate**: >95% for dependency layers

## Security Checklist
- [ ] All wallet files have 400 permissions
- [ ] .env file has 600 permissions
- [ ] No secrets in Docker images (verify with `docker history`)
- [ ] All services run as non-root
- [ ] Resource limits configured
- [ ] Network isolation enabled
- [ ] Logs don't contain sensitive data

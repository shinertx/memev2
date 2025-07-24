# ...existing code...

## Architecture Overview
# ...existing code...

### Canonical Docker Builds
All services are built using one of two universal Dockerfiles, ensuring consistency and maintainability.
- **`Dockerfile.rust`**: A multi-stage, performance-tuned (`jemalloc`) file for all Rust services.
- **`Dockerfile.python`**: A universal file for all Python services.
- **`docker-compose.yml`**: Single source of truth - NO OTHER COMPOSE FILES EXIST.

## Key Development Commands

### Deployment & Operations (Unified)
The entire system is managed via a single, canonical Compose file.
```bash
# Build all services using the canonical Dockerfiles
docker compose build --parallel

# Deploy the entire stack (trading + observability)
docker compose up -d

# Check status of all services
docker compose ps

# View logs for a specific service
docker compose logs -f executor

# IMPORTANT: There is NO docker-compose.prod.yml or docker-compose.dev.yml anymore!
```

### Configuration Management
- **Primary Config**: `docker-compose.yml` is the ONLY compose file.
- **Dockerfiles**: Only `Dockerfile.rust` and `Dockerfile.python` exist.
- **No Legacy Files**: Do not create service-specific Dockerfiles or alternate compose files.

# ...existing code...
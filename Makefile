.PHONY: all build test deploy clean help check-cargo cache-deps verify-cache perf-build cargo-update cargo-fix bench profile-build pgo-optimize up down logs backup monitor health

# Build configuration
DOCKER_BUILDKIT := 1
COMPOSE_DOCKER_CLI_BUILD := 1
COMPOSE := docker compose
CARGO_CHEF_VERSION := 0.1.62
RUST_VERSION := 1.75

# Cache configuration for maximum efficiency
export BUILDKIT_INLINE_CACHE := 1
export DOCKER_BUILDKIT := 1
export COMPOSE_DOCKER_CLI_BUILD := 1

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

all: check-cargo test perf-build ## Run tests and build all images with performance optimizations

check-cargo: ## Verify all Cargo.toml files are valid and dependencies are optimal
	@echo "Checking Cargo.toml files..."
	@find . -name "Cargo.toml" -type f | while read f; do \
		echo "Validating $$f"; \
		cargo verify-project --manifest-path $$f || exit 1; \
	done
	@echo "Checking for outdated dependencies..."
	cargo outdated --root-deps-only || true
	@echo "Checking for security vulnerabilities..."
	cargo audit || true

cache-deps: ## Pre-cache all Rust dependencies for faster builds
	@echo "Pre-caching Rust dependencies with cargo-chef..."
	docker build --target planner -f Dockerfile.rust -t memesnipe-deps-cache .
	docker build --target builder -f Dockerfile.rust -t memesnipe-builder-cache .

test: ## Run all tests with optimal flags
	@echo "Running Rust tests..."
	cargo fmt --all -- --check
	cargo clippy --all -- -D warnings
	RUSTFLAGS="-C target-cpu=native" cargo test --all --release

perf-build: ## Build all Docker images with maximum performance and caching
	@echo "Building all services with performance optimizations..."
	$(COMPOSE) build --parallel \
		--build-arg BUILDKIT_INLINE_CACHE=1 \
		--build-arg CARGO_CHEF_VERSION=$(CARGO_CHEF_VERSION) \
		--build-arg RUST_VERSION=$(RUST_VERSION)

build: perf-build ## Alias for perf-build

verify-cache: ## Verify Docker build cache is being used effectively
	@echo "Analyzing Docker build cache..."
	docker buildx du --verbose
	@echo "\nCache statistics:"
	docker system df

up: ## Start all services
	$(COMPOSE) up -d
	@echo "Waiting for services to be healthy..."
	@sleep 5
	$(COMPOSE) ps

down: ## Stop all services
	$(COMPOSE) down

logs: ## Show logs from all services
	$(COMPOSE) logs -f

clean: ## Clean build artifacts and volumes
	$(COMPOSE) down -v
	cargo clean
	find . -type d -name "__pycache__" -exec rm -rf {} +
	find . -type f -name "*.pyc" -delete

backup: ## Backup database and Redis
	@mkdir -p backups
	@echo "Backing up database..."
	$(COMPOSE) exec -T dashboard sqlite3 /app/shared/trades_v18.db ".backup /tmp/backup.db"
	docker cp memesnipe-dashboard:/tmp/backup.db ./backups/trades_$$(date +%Y%m%d_%H%M%S).db
	@echo "Backing up Redis..."
	$(COMPOSE) exec -T redis redis-cli BGSAVE
	@sleep 2
	docker cp memesnipe-redis:/data/dump.rdb ./backups/redis_$$(date +%Y%m%d_%H%M%S).rdb

monitor: ## Open monitoring dashboards
	@echo "Opening monitoring dashboards..."
	@echo "Dashboard: http://localhost"
	@echo "Grafana: http://localhost:3000"
	@echo "Prometheus: http://localhost:9090"

health: ## Check health of all services
	@echo "Checking service health..."
	@$(COMPOSE) ps
	@echo "\nChecking endpoints..."
	@for port in 9091 9092 9093 9094 9095 8080 80 3000 9090; do \
		echo -n "Port $$port: "; \
		curl -s -o /dev/null -w "%{http_code}" http://localhost:$$port/health 2>/dev/null || echo "N/A"; \
	done

cargo-update: ## Update Cargo.lock with latest compatible versions
	@echo "Updating Cargo dependencies..."
	cargo update
	@echo "Updating workspace dependencies..."
	find . -name "Cargo.toml" -type f | while read f; do \
		dir=$$(dirname $$f); \
		if [ -f "$$dir/Cargo.lock" ]; then \
			echo "Updating $$dir/Cargo.lock"; \
			(cd $$dir && cargo update); \
		fi; \
	done

cargo-fix: ## Fix common Cargo/Rust issues and optimize
	@echo "Running cargo fix..."
	cargo fix --allow-dirty --allow-staged
	@echo "Cleaning unused dependencies..."
	cargo machete
	@echo "Optimizing Cargo.toml files..."
	find . -name "Cargo.toml" -type f | while read f; do \
		echo "Optimizing $$f"; \
		cargo diet --manifest-path $$f || true; \
	done

bench: ## Run performance benchmarks
	@echo "Running performance benchmarks..."
	RUSTFLAGS="-C target-cpu=native" cargo bench --all

profile-build: ## Build with profiling data for PGO
	@echo "Building with profile-guided optimization..."
	RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" $(COMPOSE) build executor
	@echo "Run your workload now, then run 'make pgo-optimize'"

pgo-optimize: ## Apply profile-guided optimization
	@echo "Applying PGO..."
	RUSTFLAGS="-Cprofile-use=/tmp/pgo-data -Cllvm-args=-pgo-warn-missing-function" $(COMPOSE) build executor

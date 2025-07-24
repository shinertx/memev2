# MemeSnipe v18 - Production Makefile

.PHONY: help deploy start stop restart status logs update backup clean health test build cargo-verify cargo-clean-deps cargo-update-smart cargo-tree cargo-bloat validate-env

# Default target
all: help

# Show help
help:
	@echo "🚀 MemeSnipe v18 - Production Trading System"
	@echo ""
	@echo "Available commands:"
	@echo "  make deploy    - Full deployment"
	@echo "  make start     - Start all services"
	@echo "  make stop      - Stop all services"
	@echo "  make restart   - Restart all services"
	@echo "  make status    - Show system status"
	@echo "  make logs      - View all logs"
	@echo "  make update    - Update from git"
	@echo "  make backup    - Create backup"
	@echo "  make clean     - Clean up resources"
	@echo "  make health    - Check service health"
	@echo "  make test      - Run tests"
	@echo "  make build     - Build Docker images"
	@echo ""
	@echo "Direct access:"
	@echo "  ./deploy.sh [command]  - Use deployment script directly"

# Deployment commands (using deploy.sh)
deploy:
	./deploy.sh deploy

start:
	./deploy.sh start

stop:
	./deploy.sh stop

restart:
	./deploy.sh restart

status:
	./deploy.sh status

logs:
	./deploy.sh logs

update:
	./deploy.sh update

backup:
	./deploy.sh backup

clean:
	./deploy.sh clean

health:
	./deploy.sh health

# Development commands
test:
	cargo fmt --all
	cargo clippy --all-targets -- -D warnings
	cargo test --all
	@echo "✅ All tests passed"

build:
	export DOCKER_BUILDKIT=1
	sudo docker compose -f docker-compose.working.yml build

cargo-verify: ## Verify all Cargo.toml files are valid
	@echo "Verifying workspace configuration..."
	@cargo metadata --no-deps > /dev/null
	@echo "Checking all workspace members..."
	@for member in $$(cargo metadata --no-deps --format-version 1 | jq -r '.workspace_members[]' | cut -d' ' -f1); do \
		echo "✓ $$member"; \
	done

cargo-clean-deps: ## Remove unused dependencies
	@echo "Installing cargo-machete..."
	@cargo install cargo-machete --locked
	@echo "Checking for unused dependencies..."
	@cargo machete

cargo-update-smart: ## Update dependencies with compatibility checks
	@echo "Installing cargo-edit..."
	@cargo install cargo-edit --locked
	@echo "Checking for updates..."
	@cargo update --dry-run
	@echo "Run 'cargo update' to apply updates"

cargo-tree: ## Show dependency tree
	@cargo tree --workspace

cargo-bloat: ## Analyze binary size
	@echo "Installing cargo-bloat..."
	@cargo install cargo-bloat --locked
	@cargo bloat --release --crates

validate-env: ## Validate .env configuration
	@chmod +x scripts/validate-env.sh
	@./scripts/validate-env.sh

setup: validate-env ## Initial setup and validation
	@echo "🚀 Setting up MemeSnipe v18..."
	@if [ ! -f .env ]; then \
		echo "Creating .env from template..."; \
		cp .env.example .env; \
		echo "⚠️  Please edit .env with your configuration"; \
		exit 1; \
	fi
	@echo "✅ Setup complete"

pre-deploy: validate-env cargo-verify ## Pre-deployment checks
	@echo "🔍 Running pre-deployment checks..."
	@echo "✅ All checks passed"

# Update the all target to include validation
all: validate-env check-cargo test perf-build ## Run validation, tests and build all images

# Legacy aliases for compatibility
up: start
down: stop
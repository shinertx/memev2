# MemeSnipe v18 - Production Makefile

.PHONY: help deploy start stop restart status logs update backup clean health test build

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

# Legacy aliases for compatibility
up: start
down: stop 
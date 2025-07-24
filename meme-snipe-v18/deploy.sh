#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Deployment functions using ONLY docker-compose.yml
deploy() {
    echo -e "${GREEN}🚀 Deploying MemeSnipe v18 (Platinum Standard)${NC}"
    
    # Ensure we're using the right Docker setup
    export DOCKER_BUILDKIT=1
    export COMPOSE_DOCKER_CLI_BUILD=1
    
    # Build with caching
    echo -e "${YELLOW}Building services...${NC}"
    docker compose build --parallel
    
    # Start infrastructure first
    echo -e "${YELLOW}Starting infrastructure...${NC}"
    docker compose up -d redis prometheus grafana
    sleep 5
    
    # Start core services
    echo -e "${YELLOW}Starting core services...${NC}"
    docker compose up -d risk_guardian wallet_guard signer
    sleep 3
    
    # Start trading services
    echo -e "${YELLOW}Starting trading services...${NC}"
    docker compose up -d executor position_manager autonomous_allocator
    
    # Start auxiliary services
    echo -e "${YELLOW}Starting auxiliary services...${NC}"
    docker compose up -d data_consumers strategy_factory dashboard
    
    # Show status
    docker compose ps
}

status() {
    echo -e "${GREEN}📊 System Status${NC}"
    docker compose ps
}

logs() {
    if [ -z "$1" ]; then
        docker compose logs -f --tail=100
    else
        docker compose logs -f --tail=100 "$1"
    fi
}

stop() {
    echo -e "${YELLOW}Stopping all services...${NC}"
    docker compose down
}

restart() {
    echo -e "${YELLOW}Restarting services...${NC}"
    stop
    sleep 3
    deploy
}

health() {
    echo -e "${GREEN}📊 Health Check${NC}"
    docker compose ps
    echo ""
    echo "Checking service endpoints..."
    for port in 9091 9092 9093 9094 9095 8080 80 3000 9090; do
        printf "Port %-5s: " "$port"
        curl -s -o /dev/null -w "%{http_code}\n" http://localhost:$port/health 2>/dev/null || echo "N/A"
    done
}

# Main command handler
case "$1" in
    deploy)
        deploy
        ;;
    status)
        status
        ;;
    logs)
        logs "$2"
        ;;
    stop)
        stop
        ;;
    restart)
        restart
        ;;
    health)
        health
        ;;
    *)
        echo "Usage: $0 {deploy|status|logs|stop|restart|health}"
        echo ""
        echo "Commands:"
        echo "  deploy  - Deploy all services"
        echo "  status  - Show service status"
        echo "  logs    - Show logs (optionally specify service)"
        echo "  stop    - Stop all services"
        echo "  restart - Restart all services"
        echo "  health  - Check service health"
        exit 1
        ;;
esac

#!/bin/bash
set -e

# 🚀 MemeSnipe v18 - Complete Deployment Script
# Consolidates all deployment, monitoring, and management functions

# Configuration
PROJECT_NAME="MemeSnipe v18"
COMPOSE_FILE="docker-compose.prod.yml"
ENV_FILE=".env"
BACKUP_DIR="backups"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
NC='\033[0m' # No Color

# Emoji for better UX
ROCKET="🚀"
CHECK="✅"
WARNING="⚠️"
ERROR="❌"
INFO="ℹ️"
GEAR="⚙️"
CHART="📊"
MONEY="💰"
SHIELD="🛡️"
FIRE="🔥"

print_header() {
    echo -e "\n${PURPLE}╔══════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${PURPLE}║${WHITE}                    ${ROCKET} MemeSnipe v18 Deployment ${ROCKET}                    ${PURPLE}║${NC}"
    echo -e "${PURPLE}║${WHITE}                     Production Trading System                     ${PURPLE}║${NC}"
    echo -e "${PURPLE}╚══════════════════════════════════════════════════════════════════╝${NC}\n"
}

print_separator() {
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
}

log_info() {
    echo -e "${INFO} ${CYAN}$1${NC}"
}

log_success() {
    echo -e "${CHECK} ${GREEN}$1${NC}"
}

log_warning() {
    echo -e "${WARNING} ${YELLOW}$1${NC}"
}

log_error() {
    echo -e "${ERROR} ${RED}$1${NC}"
}

# Check if running on GCP
check_gcp_environment() {
    if curl -s metadata.google.internal -m 5 >/dev/null 2>&1; then
        echo "true"
    else
        echo "false"
    fi
}

# Install system dependencies
install_dependencies() {
    log_info "Installing system dependencies..."
    
    # Update package list
    sudo apt-get update -qq
    
    # Install required packages
    sudo apt-get install -y \
        curl \
        wget \
        git \
        unzip \
        software-properties-common \
        apt-transport-https \
        ca-certificates \
        gnupg \
        lsb-release \
        jq \
        redis-tools \
        net-tools \
        htop \
        python3 \
        python3-pip \
        build-essential
    
    log_success "System dependencies installed"
}

# Install Docker
install_docker() {
    if ! command -v docker &> /dev/null; then
        log_info "Installing Docker..."
        
        # Add Docker's official GPG key
        curl -fsSL https://download.docker.com/linux/debian/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
        
        # Add Docker repository
        echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/debian $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
        
        # Install Docker
        sudo apt-get update -qq
        sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
        
        # Add user to docker group
        sudo usermod -aG docker $USER
        
        log_success "Docker installed successfully"
    else
        log_success "Docker already installed"
    fi
}

# Install Rust (for building services)
install_rust() {
    if ! command -v rustc &> /dev/null; then
        log_info "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        log_success "Rust installed successfully"
    else
        log_success "Rust already installed"
    fi
}

# Setup environment
setup_environment() {
    log_info "Setting up environment..."
    
    # Create necessary directories
    mkdir -p shared backups logs
    
    # Set proper permissions
    chmod +x deploy.sh
    
    # Create .env if it doesn't exist
    if [ ! -f "$ENV_FILE" ]; then
        log_warning ".env file not found, creating from template..."
        create_env_file
    fi
    
    log_success "Environment setup complete"
}

# Create .env file with production settings
create_env_file() {
    log_info "Creating production .env file..."
    
    cat > "$ENV_FILE" << 'EOF'
# MemeSnipe v18 - Production Environment Configuration
# 🚀 LIVE TRADING READY - Auto-Generated

# ============================================================================
# 🚨 CRITICAL SAFETY SETTINGS 🚨
# ============================================================================
PAPER_TRADING_MODE=true  # Set to 'false' for live trading
SECURITY_MODE=true
ENABLE_EMERGENCY_STOP=true
MAX_DAILY_LOSS_USD=500.00

# ============================================================================
# 🔑 WALLET CONFIGURATION
# ============================================================================
WALLET_KEYPAIR_FILENAME=my_wallet.json
JITO_AUTH_KEYPAIR_FILENAME=jito_auth_key.json

# ============================================================================
# 🌐 RPC ENDPOINTS
# ============================================================================
SOLANA_RPC_URL=https://mainnet.helius-rpc.com/?api-key=cb0b0046-e7ed-4538-b1ce-eb477265901a
JITO_RPC_URL=https://mainnet.block-engine.jito.wtf/api
SIGNER_URL=http://signer:8989

# ============================================================================
# 🔗 API KEYS (UPDATE WITH YOUR KEYS)
# ============================================================================
HELIUS_API_KEY=cb0b0046-e7ed-4538-b1ce-eb477265901a
PYTH_API_KEY=YOUR_PYTH_API_KEY_HERE
TWITTER_BEARER_TOKEN=AAAAAAAAAAAAAAAAAAAAAD4a2gEAAAAAb6hOFYouWlfBAJQ9ppSQXdiXpFc%3DyENww48woIwib9kGRKaZLQkEE0u75bItAybPkUbRA4Bp8zMABz
OPENAI_API_KEY=YOUR_OPENAI_API_KEY_HERE
GROK_API_KEY=YOUR_GROK_API_KEY_HERE
DRIFT_API_URL=https://api.drift.trade

# ============================================================================
# 💰 RISK MANAGEMENT
# ============================================================================
GLOBAL_MAX_POSITION_USD=250.00
PORTFOLIO_STOP_LOSS_PERCENT=25.0
TRAILING_STOP_LOSS_PERCENT=15.0
MAX_CONCURRENT_TRADES=5
MAX_DAILY_TRADES=25
POSITION_SIZE_PERCENT=10
KELLY_FRACTION=0.5
MIN_BET_LAMPORTS=1000000
MAX_BET_PERCENTAGE=5

# ============================================================================
# 🎯 STRATEGY CONFIGURATION
# ============================================================================
ENABLE_BRIDGE_FLOW_MONITORING=true
ENABLE_FUNDING_RATE_ANALYSIS=true
ENABLE_DEPTH_ANALYSIS=true
ENABLE_ONCHAIN_MONITORING=true
ENABLE_PRICE_FEEDS=true
ENABLE_TRADE_EXECUTOR=true
ENABLE_POSITION_MANAGER=true
ENABLE_META_ALLOCATOR=true

# ============================================================================
# ⚡ EXECUTION SETTINGS
# ============================================================================
JUPITER_API_URL=https://quote-api.jup.ag/v6
SLIPPAGE_BPS=50
JITO_TIP_LAMPORTS=100000
JITO_TIP_LAMPORTS_MIN=50000
JITO_TIP_LAMPORTS_MAX=200000
TRADE_TIP=50000

# ============================================================================
# 📊 MONITORING & PORT CONFIGURATION
# ============================================================================
LOG_LEVEL=info
MAIN_URL=http://localhost
DASHBOARD_URL=http://localhost
API_BASE_URL=http://localhost/api
DASHBOARD_PORT=8080
SIGNER_PORT=8989
REDIS_PORT=6379
SIGNER_URL=http://signer:8989
REDIS_URL=redis://redis:6379

# ============================================================================
# 💾 DATA STORAGE
# ============================================================================
DATABASE_PATH=/app/shared/trades_v17.db
REDIS_URL=redis://redis:6379
REPLAY_DB_PATH=shared/replay.db

# ============================================================================
# 🔔 NOTIFICATIONS (UPDATE WITH YOUR TOKENS)
# ============================================================================
TELEGRAM_BOT_TOKEN=YOUR_BOT_TOKEN_HERE
TELEGRAM_CHAT_ID=YOUR_CHAT_ID_HERE
DISCORD_WEBHOOK_URL=YOUR_DISCORD_WEBHOOK_HERE

# ============================================================================
# 🛡️ SAFETY FEATURES
# ============================================================================
ENABLE_CIRCUIT_BREAKER=true
CIRCUIT_BREAKER_LOSS_THRESHOLD=100.00
ENABLE_POSITION_LIMITS=true
MAX_DRAWDOWN_PERCENT=15.0
RATE_LIMIT_ORDERS_PER_MIN=30

# ============================================================================
# 🏗️ FEATURE FLAGS
# ============================================================================
MARGINFI_VAULT=true
HELIUS_PRO=false
DEBUG=false
MOCK_TRADES=false
EOF
    
    log_success ".env file created with production settings"
}

# Generate wallet if needed
generate_wallet() {
    if [ ! -f "my_wallet.json" ]; then
        log_info "Generating wallet keypair..."
        
        python3 << 'EOF'
import json
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

# Generate Ed25519 keypair
private_key = Ed25519PrivateKey.generate()
private_bytes = private_key.private_bytes_raw()
public_bytes = private_key.public_key().public_bytes_raw()

# Combine for Solana format (64 bytes total)
keypair_bytes = list(private_bytes + public_bytes)

# Save to file
with open('my_wallet.json', 'w') as f:
    json.dump(keypair_bytes, f)

print("Wallet generated successfully")
EOF
        
        log_success "Wallet keypair generated"
    else
        log_success "Wallet already exists"
    fi
}

# Build Docker images
build_images() {
    log_info "Building Docker images..."
    
    export DOCKER_BUILDKIT=1
    
    # Build all services
    sudo docker compose -f "$COMPOSE_FILE" build --parallel
    
    log_success "Docker images built successfully"
}

# Start services
start_services() {
    log_info "Starting all services..."
    
    # Start core services first
    sudo docker compose -f "$COMPOSE_FILE" up -d redis
    sleep 5
    
    # Start data pipeline
    sudo docker compose -f "$COMPOSE_FILE" up -d \
        bridge_consumer \
        funding_consumer \
        depth_consumer \
        helius_rpc_price_consumer \
        onchain_consumer \
        strategy_factory
    sleep 10
    
    # Start trading services
    sudo docker compose -f "$COMPOSE_FILE" up -d signer
    sleep 5
    
    # Start dashboard and proxy
    sudo docker compose -f "$COMPOSE_FILE" up -d dashboard nginx
    sleep 5
    
    log_success "All services started"
}

# Check service health
check_health() {
    log_info "Checking service health..."
    
    local all_healthy=true
    
    # Check if services are running
    services=("redis" "signer" "dashboard" "nginx" "bridge_consumer" "strategy_factory")
    
    for service in "${services[@]}"; do
        if sudo docker compose -f "$COMPOSE_FILE" ps "$service" | grep -q "Up"; then
            log_success "$service is running"
        else
            log_error "$service is not running"
            all_healthy=false
        fi
    done
    
    # Check HTTP endpoints
    sleep 10
    if curl -s http://localhost/health >/dev/null; then
        log_success "Dashboard is accessible"
    else
        log_error "Dashboard is not accessible"
        all_healthy=false
    fi
    
    if [ "$all_healthy" = true ]; then
        log_success "All services are healthy"
        return 0
    else
        log_error "Some services are unhealthy"
        return 1
    fi
}

# Show system status
show_status() {
    print_separator
    echo -e "${CHART} ${WHITE}MEMESNIPE v18 SYSTEM STATUS${NC}"
    print_separator
    
    # Service status
    echo -e "\n${GEAR} ${BLUE}Services:${NC}"
    sudo docker compose -f "$COMPOSE_FILE" ps --format "table {{.Service}}\t{{.Status}}\t{{.Ports}}"
    
    # Health check
    echo -e "\n${SHIELD} ${BLUE}Health Check:${NC}"
    if curl -s http://localhost/health | jq . 2>/dev/null; then
        echo
    else
        echo "Health endpoint not responding"
    fi
    
    # System resources
    echo -e "\n${FIRE} ${BLUE}System Resources:${NC}"
    echo "Memory: $(free -h | awk '/^Mem:/ {print $3 "/" $2}')"
    echo "Disk: $(df -h / | awk '/\// {print $3 "/" $2 " (" $5 " used)"}')"
    echo "Load: $(uptime | awk -F'load average:' '{print $2}')"
    
    # URLs
    echo -e "\n${ROCKET} ${BLUE}Access URLs:${NC}"
    echo "Dashboard: http://localhost"
    echo "Health: http://localhost/health"
    echo "API: http://localhost/api/"
    
    print_separator
}

# View logs
view_logs() {
    local service=${1:-""}
    
    if [ -n "$service" ]; then
        log_info "Showing logs for $service..."
        sudo docker compose -f "$COMPOSE_FILE" logs -f --tail=50 "$service"
    else
        log_info "Showing logs for all services..."
        sudo docker compose -f "$COMPOSE_FILE" logs -f --tail=20
    fi
}

# Stop services
stop_services() {
    log_info "Stopping all services..."
    sudo docker compose -f "$COMPOSE_FILE" down
    log_success "All services stopped"
}

# Restart services
restart_services() {
    log_info "Restarting services..."
    stop_services
    sleep 5
    start_services
    log_success "Services restarted"
}

# Update system
update_system() {
    log_info "Updating system..."
    
    # Pull latest code
    git pull origin main
    
    # Rebuild and restart
    build_images
    restart_services
    
    log_success "System updated"
}

# Backup data
backup_data() {
    log_info "Creating backup..."
    
    local backup_name="backup_$(date +%Y%m%d_%H%M%S)"
    mkdir -p "$BACKUP_DIR/$backup_name"
    
    # Backup database
    if [ -f "shared/trades_v17.db" ]; then
        cp shared/trades_v17.db "$BACKUP_DIR/$backup_name/"
    fi
    
    # Backup wallet
    if [ -f "my_wallet.json" ]; then
        cp my_wallet.json "$BACKUP_DIR/$backup_name/"
    fi
    
    # Backup configuration
    cp .env "$BACKUP_DIR/$backup_name/"
    
    # Create archive
    tar -czf "$BACKUP_DIR/$backup_name.tar.gz" -C "$BACKUP_DIR" "$backup_name"
    rm -rf "$BACKUP_DIR/$backup_name"
    
    log_success "Backup created: $BACKUP_DIR/$backup_name.tar.gz"
}

# Clean up resources
cleanup() {
    log_info "Cleaning up resources..."
    
    # Stop all services
    sudo docker compose -f "$COMPOSE_FILE" down -v
    
    # Remove unused Docker resources
    sudo docker system prune -f
    
    log_success "Cleanup complete"
}

# Main deployment function
deploy() {
    print_header
    
    log_info "Starting MemeSnipe v18 deployment..."
    
    # Install dependencies
    install_dependencies
    install_docker
    install_rust
    
    # Setup environment
    setup_environment
    generate_wallet
    
    # Build and start
    build_images
    start_services
    
    # Health check
    if check_health; then
        print_separator
        log_success "🎉 DEPLOYMENT SUCCESSFUL! 🎉"
        echo
        echo -e "${MONEY} ${GREEN}Your MemeSnipe v18 trading system is now running!${NC}"
        echo
        echo -e "${ROCKET} ${WHITE}Access your dashboard at: ${CYAN}http://localhost${NC}"
        echo -e "${SHIELD} ${WHITE}Health check: ${CYAN}http://localhost/health${NC}"
        echo -e "${CHART} ${WHITE}API endpoints: ${CYAN}http://localhost/api/${NC}"
        echo
        echo -e "${WARNING} ${YELLOW}Remember to:${NC}"
        echo "  1. Update API keys in .env file"
        echo "  2. Fund your wallet with SOL"
        echo "  3. Test in paper mode first"
        echo "  4. Set PAPER_TRADING_MODE=false when ready"
        print_separator
    else
        log_error "Deployment failed. Check logs for details."
        exit 1
    fi
}

# Show help
show_help() {
    print_header
    echo -e "${WHITE}Usage: ./deploy.sh [COMMAND]${NC}\n"
    echo -e "${BLUE}Commands:${NC}"
    echo "  deploy          Full deployment (default)"
    echo "  start           Start all services"
    echo "  stop            Stop all services"
    echo "  restart         Restart all services"
    echo "  status          Show system status"
    echo "  logs [service]  View logs (optionally for specific service)"
    echo "  update          Update system from git"
    echo "  backup          Create data backup"
    echo "  clean           Clean up resources"
    echo "  health          Check service health"
    echo "  help            Show this help"
    echo
    echo -e "${YELLOW}Examples:${NC}"
    echo "  ./deploy.sh                 # Full deployment"
    echo "  ./deploy.sh status          # Check system status"
    echo "  ./deploy.sh logs signer     # View signer logs"
    echo "  ./deploy.sh restart         # Restart all services"
    echo
}

# Main script logic
case "${1:-deploy}" in
    "deploy")
        deploy
        ;;
    "start")
        start_services
        ;;
    "stop")
        stop_services
        ;;
    "restart")
        restart_services
        ;;
    "status")
        show_status
        ;;
    "logs")
        view_logs "$2"
        ;;
    "update")
        update_system
        ;;
    "backup")
        backup_data
        ;;
    "clean")
        cleanup
        ;;
    "health")
        check_health
        ;;
    "help"|"-h"|"--help")
        show_help
        ;;
    *)
        log_error "Unknown command: $1"
        show_help
        exit 1
        ;;
esac

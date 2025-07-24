#!/bin/bash
"""
Post-deployment setup script for MemeSnipe v18
Runs database optimizations and validates system health
"""

echo "🚀 MemeSnipe v18 Post-Deployment Setup"
echo "======================================"

# Check if we're in the right directory
if [[ ! -f "docker-compose.working.yml" ]]; then
    echo "❌ Error: docker-compose.working.yml not found"
    echo "Please run this script from the meme-snipe-v18 directory"
    exit 1
fi

echo "📁 Working directory: $(pwd)"

# Set proper permissions on wallet files
echo "🔐 Setting wallet file permissions..."
if [[ -f "my_wallet.json" ]]; then
    chmod 400 my_wallet.json
    echo "✅ Set permissions on my_wallet.json"
else
    echo "⚠️  Warning: my_wallet.json not found"
fi

if [[ -f "jito_auth_key.json" ]]; then
    chmod 400 jito_auth_key.json
    echo "✅ Set permissions on jito_auth_key.json"
else
    echo "⚠️  Warning: jito_auth_key.json not found"
fi

# Create logs directory if it doesn't exist
mkdir -p logs
echo "📂 Created logs directory"

# Run database optimization
echo "🔧 Optimizing database..."
if [[ -f "scripts/optimize_database.py" ]]; then
    python3 scripts/optimize_database.py
    if [[ $? -eq 0 ]]; then
        echo "✅ Database optimization completed"
    else
        echo "⚠️  Database optimization had warnings (check logs)"
    fi
else
    echo "⚠️  Database optimization script not found"
fi

# Check .env configuration
echo "⚙️  Checking configuration..."
if [[ -f ".env" ]]; then
    echo "✅ .env file found"
    
    # Check for required configurations
    if grep -q "PAPER_TRADING_MODE=true" .env; then
        echo "✅ Paper trading mode enabled"
    else
        echo "⚠️  PAPER_TRADING_MODE not set to true"
    fi
    
    if grep -q "HELIUS_API_KEY=" .env; then
        echo "✅ Helius API key configured"
    else
        echo "⚠️  HELIUS_API_KEY not found in .env"
    fi
else
    echo "❌ .env file not found - copy from .env.example"
    exit 1
fi

# Start services using working configuration
echo "🐳 Starting services with enhanced configuration..."
docker-compose -f docker-compose.working.yml up -d

# Wait for services to start
echo "⏳ Waiting for services to initialize..."
sleep 30

# Health check
echo "🏥 Performing health checks..."

# Check Redis
if docker-compose -f docker-compose.working.yml exec -T redis redis-cli ping | grep -q "PONG"; then
    echo "✅ Redis is healthy"
else
    echo "❌ Redis health check failed"
fi

# Check if executor is running
if docker-compose -f docker-compose.working.yml ps executor | grep -q "Up"; then
    echo "✅ Executor service is running"
else
    echo "❌ Executor service failed to start"
fi

# Check if dashboard is accessible
if curl -f -s http://localhost/health > /dev/null; then
    echo "✅ Dashboard is accessible"
else
    echo "⚠️  Dashboard health check failed"
fi

# Display service status
echo ""
echo "📊 Service Status:"
echo "=================="
docker-compose -f docker-compose.working.yml ps

echo ""
echo "🎯 Setup Complete!"
echo "=================="
echo "• Dashboard: http://localhost"
echo "• Health Check: http://localhost/health"
echo "• Metrics: http://localhost/metrics"
echo ""
echo "📋 Next Steps:"
echo "1. Obtain Helius Premium API key ($499/month)"
echo "2. Fund Jito tip wallet for MEV protection"
echo "3. Configure Twitter API for social signals"
echo "4. Monitor logs: ./deploy.sh logs [service]"
echo ""
echo "⚠️  Remember: System is in PAPER TRADING mode"
echo "Set PAPER_TRADING_MODE=false in .env for live trading"

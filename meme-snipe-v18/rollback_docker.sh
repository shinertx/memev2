#!/bin/bash
# ROLLBACK DOCKER SCRIPT
# Created: July 22, 2025
# Use this script to rollback to the working container state

echo "🔄 DOCKER ROLLBACK INITIATED..."
echo "Timestamp: $(date)"

# Stop current containers
echo "📦 Stopping current containers..."
sudo docker compose down

# Rollback to working images
echo "🏷️ Rolling back to working images..."
sudo docker tag local/meme-snipe:signer-96fab07ff849 local/meme-snipe:signer 2>/dev/null || echo "Signer image rollback skipped"
sudo docker tag local/meme-snipe:data-consumer-4fcba0379048 local/meme-snipe:data-consumer 2>/dev/null || echo "Data consumer rollback skipped"
sudo docker tag local/meme-snipe:dashboard-a12f73bb86a1 local/meme-snipe:dashboard 2>/dev/null || echo "Dashboard rollback skipped"
sudo docker tag local/meme-snipe:strategy-factory-75ed9817bd18 local/meme-snipe:strategy-factory 2>/dev/null || echo "Strategy factory rollback skipped"

# Restart with working compose file
echo "🚀 Starting rollback containers..."
sudo docker compose -f docker-compose.working.yml up -d

echo "✅ Rollback complete!"
echo "🌐 Check http://localhost for dashboard"
echo "📊 Run 'sudo docker ps' to verify containers"

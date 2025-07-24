#!/bin/bash

echo "Cleaning up old Docker files..."

# Remove old Docker files
rm -f docker-compose.prod.yml
rm -f docker-compose.dev.yml
rm -f Dockerfile
find . -name "Dockerfile" -type f -delete
find . -name "docker-compose.yml" -type f -not -path "./docker-compose.yml" -delete

# Remove old build artifacts
docker system prune -f
docker builder prune -f

echo "Docker cleanup complete!"
echo "New Docker files:"
echo "  - Dockerfile.rust (for all Rust services)"
echo "  - Dockerfile.python (for all Python services)"
echo "  - docker-compose.yml (single source of truth)"
echo "  - prometheus.yml (monitoring config)"

echo ""
echo "To deploy with the new setup:"
echo "  cd /home/benjaminjones/memev2/meme-snipe-v18"
echo "  make all"
echo "  make up"

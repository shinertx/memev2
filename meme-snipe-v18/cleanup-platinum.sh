#!/bin/bash

echo "🧹 Cleaning up files not aligned with platinum Docker standard..."

# Create backup directory
mkdir -p old-files-backup

# Move old Dockerfiles
echo "📦 Removing old Dockerfiles..."
mv Dockerfile.rust.nightly old-files-backup/ 2>/dev/null || true
mv Dockerfile.rust.simple old-files-backup/ 2>/dev/null || true

# Move old compose files
echo "📄 Removing old docker-compose files..."
mv docker-compose.prod.yml old-files-backup/ 2>/dev/null || true
mv docker-compose.rollback-backup.yml old-files-backup/ 2>/dev/null || true
mv docker-compose.security.yml old-files-backup/ 2>/dev/null || true
mv docker-compose.upgraded.yml old-files-backup/ 2>/dev/null || true
mv docker-compose.working.yml old-files-backup/ 2>/dev/null || true

# Move rollback files
echo "🔄 Removing rollback files..."
mv rollback_docker.sh old-files-backup/ 2>/dev/null || true
mv rollback_docker_state.txt old-files-backup/ 2>/dev/null || true
mv rollback_images.txt old-files-backup/ 2>/dev/null || true
mv rollback_info.txt old-files-backup/ 2>/dev/null || true

# Move build artifacts
echo "🔨 Removing build artifacts..."
mv meta-allocator-build-Cargo.toml old-files-backup/ 2>/dev/null || true
mv signer-build-Cargo.toml old-files-backup/ 2>/dev/null || true

# Update deploy.sh to remove references to old files
echo "📝 Updating deploy.sh..."
if [ -f deploy.sh ]; then
    sed -i 's/docker-compose\.prod\.yml/docker-compose.yml/g' deploy.sh
    sed -i 's/docker-compose -f [^ ]* /docker compose /g' deploy.sh
fi

echo "✅ Cleanup complete!"
echo ""
echo "📁 Files backed up to: old-files-backup/"
echo ""
echo "🏆 Platinum Docker Standard Files:"
echo "   ✓ Dockerfile.rust (all Rust services)"
echo "   ✓ Dockerfile.python (all Python services)"
echo "   ✓ docker-compose.yml (single source of truth)"
echo "   ✓ prometheus.yml (monitoring config)"
echo "   ✓ Makefile (with optimized commands)"
echo ""
echo "📋 Next steps:"
echo "   1. Review and update any scripts in scripts/ directory"
echo "   2. Update .gitignore to exclude old-files-backup/"
echo "   3. Commit the cleaned structure"

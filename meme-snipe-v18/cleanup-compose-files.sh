#!/bin/bash

echo "🧹 Cleaning up multiple docker-compose files..."

# Backup old compose files before removal
mkdir -p docker-backup
mv docker-compose.prod.yml docker-backup/ 2>/dev/null || true
mv docker-compose.rollback-backup.yml docker-backup/ 2>/dev/null || true
mv docker-compose.security.yml docker-backup/ 2>/dev/null || true
mv docker-compose.upgraded.yml docker-backup/ 2>/dev/null || true
mv docker-compose.working.yml docker-backup/ 2>/dev/null || true

echo "✅ Old compose files backed up to docker-backup/"
echo "📄 Only docker-compose.yml remains as the single source of truth"

# Update any scripts that reference old compose files
find . -type f -name "*.sh" -exec grep -l "docker-compose.prod.yml" {} \; | while read file; do
    echo "Updating $file to use docker-compose.yml..."
    sed -i 's/docker-compose.prod.yml/docker-compose.yml/g' "$file"
done

echo "✨ Cleanup complete! Use 'docker compose' (no file specification needed)"

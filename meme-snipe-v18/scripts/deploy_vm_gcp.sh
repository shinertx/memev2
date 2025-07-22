#!/usr/bin/env bash
set -euo pipefail

# ──────────────────────────────────────────────────────────────
#  MemeSnipe v18 – GCP VM Deploy Helper (2025-07 revision)
# ──────────────────────────────────────────────────────────────
#  • Creates/updates a Debian-11 VM
#  • Installs Docker CE + compose-plugin (BuildKit enabled)
#  • Copies this repo and runs docker compose up -d
#  • Idempotent: rerun to rebuild / restart
# ──────────────────────────────────────────────────────────────

# ── User-tunable settings ─────────────────────────────────────
PROJECT_ID=$(gcloud config get-value project)
VM_NAME="meme-snipe-v18-vm"
ZONE="us-central1-a"
MACHINE_TYPE="e2-standard-4"
DISK_SIZE="30GB"
IMAGE_FAMILY="debian-11"
IMAGE_PROJECT="debian-cloud"
REMOTE_DIR="/opt/meme-snipe-v18"

# ── Verify prerequisites ──────────────────────────────────────
command -v gcloud >/dev/null || { echo "❌ gcloud CLI not found"; exit 1; }
command -v docker >/dev/null || { echo "❌ docker CLI not found"; exit 1; }

# ── Load env file (must exist) ────────────────────────────────
if [[ ! -f .env ]]; then
  echo "❌ .env missing. Create it first (see .env.example)."
  if [[ -f .env.example ]]; then
    echo "📝 Copying .env.example to .env..."
    cp .env.example .env
  else
    exit 1
  fi
fi
# shellcheck disable=SC1091
source .env

[[ -f "${WALLET_KEYPAIR_FILENAME:?}" && -f "${JITO_AUTH_KEYPAIR_FILENAME:?}" ]] || {
  echo "❌ Keypair files missing."; exit 1; }

# ── Helper: push code to VM safely (rsync via gcloud SSH) ─────
sync_repo() {
  echo "📦 Syncing repo to VM…"
  # Lightweight rsync instead of tar/scp; excludes build artefacts
  gcloud compute ssh "$VM_NAME" --zone "$ZONE" -- \
    "sudo mkdir -p ${REMOTE_DIR} && sudo chown \$USER: \$USER ${REMOTE_DIR}"
  rsync -az --delete --exclude '.git' --exclude 'target' --exclude '__pycache__' \
        ./ "gcloud:${VM_NAME}:${REMOTE_DIR}" --rsync-path="sudo rsync"
}

# ── Create VM if needed ───────────────────────────────────────
if ! gcloud compute instances describe "$VM_NAME" --zone "$ZONE" --quiet &>/dev/null; then
  echo "🔨 Creating VM $VM_NAME …"
  gcloud compute instances create "$VM_NAME" \
    --project "$PROJECT_ID" \
    --zone "$ZONE" \
    --machine-type "$MACHINE_TYPE" \
    --boot-disk-size "$DISK_SIZE" \
    --image-family "$IMAGE_FAMILY" \
    --image-project "$IMAGE_PROJECT" \
    --tags http-server,https-server \
    --metadata startup-script='#!/bin/bash
        set -e
        apt-get update -qq
        apt-get install -y --no-install-recommends \
            ca-certificates curl gnupg lsb-release
        mkdir -m 0755 -p /etc/apt/keyrings
        curl -fsSL https://download.docker.com/linux/debian/gpg \
          | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
        echo \
          "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] \
          https://download.docker.com/linux/debian $(lsb_release -cs) stable" \
          > /etc/apt/sources.list.d/docker.list
        apt-get update -qq
        apt-get install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin git
        usermod -aG docker $USER
        mkdir -p /etc/docker
        echo "{\"features\":{\"buildkit\":true}}" > /etc/docker/daemon.json
        systemctl restart docker
        echo "✅ Docker installed (BuildKit on)"
    '
  # Allow some time for the startup-script to finish
  echo "⏳ Waiting 90 s for Docker setup…"; sleep 90
fi

# ── Sync code & build ─────────────────────────────────────────
sync_repo

echo "🐳 Building images (BuildKit)…"
gcloud compute ssh "$VM_NAME" --zone "$ZONE" -- \
  "cd ${REMOTE_DIR} && \
   export DOCKER_BUILDKIT=1 COMPOSE_DOCKER_CLI_BUILD=1 && \
   sudo docker compose pull --quiet || true && \
   sudo docker compose build --parallel"

echo "🚀 Starting services…"
gcloud compute ssh "$VM_NAME" --zone "$ZONE" -- \
  "cd ${REMOTE_DIR} && sudo docker compose up -d"

# ── Firewall (open 8080 & 9184 once) ──────────────────────────
FIREWALL="meme-snipe-v18-access"
if ! gcloud compute firewall-rules describe "$FIREWALL" --quiet &>/dev/null; then
  echo "🔓 Creating firewall rule $FIREWALL"
  gcloud compute firewall-rules create "$FIREWALL" \
    --allow tcp:8080,tcp:9184 \
    --target-tags http-server \
    --description "MemeSnipe Dashboard & Prom metrics"
fi

# ── Done ───────────────────────────────────────────────────────
IP=$(gcloud compute instances describe "$VM_NAME" --zone "$ZONE" \
     --format='get(networkInterfaces[0].accessConfigs[0].natIP)')
cat <<EOF

🎉  Deployment complete!
┌────────────────────────────────────────────┐
│ Dashboard   : http://$IP:8080              │
│ Prometheus  : http://$IP:9184              │
│ SSH         : gcloud compute ssh $VM_NAME --zone $ZONE │
└────────────────────────────────────────────┘
EOF

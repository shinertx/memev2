#!/bin/bash
set -e

# Enhanced deployment script with proper error handling

# Configuration
PROJECT_ID=$(gcloud config get-value project)
VM_NAME="meme-snipe-v18-vm2"
ZONE="us-central1-a"
MACHINE_TYPE="e2-standard-4"
DISK_SIZE="50GB"
IMAGE_FAMILY="debian-11"
IMAGE_PROJECT="debian-cloud"
REPO_DIR="/opt/meme-snipe-v18"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}🚀 Deploying MemeSnipe v18 to GCP...${NC}"
echo "Project: $PROJECT_ID | VM: $VM_NAME | Zone: $ZONE"

# Validate environment
check_requirements() {
    echo -e "${YELLOW}Checking requirements...${NC}"
    
    if [ ! -f ".env" ]; then
        echo -e "${RED}❌ '.env' file not found!${NC}"
        exit 1
    fi
    
    source .env
    
    if [ ! -f "$WALLET_KEYPAIR_FILENAME" ] || [ ! -f "$JITO_AUTH_KEYPAIR_FILENAME" ]; then
        echo -e "${RED}❌ Wallet files missing!${NC}"
        exit 1
    fi
    
    # Create root Cargo.toml if missing
    if [ ! -f "Cargo.toml" ]; then
        echo -e "${YELLOW}Creating root Cargo.toml...${NC}"
        cat > Cargo.toml << 'EOF'
[workspace]
members = [
    "executor",
    "signer", 
    "meta_allocator",
    "position_manager",
    "shared-models"
]
resolver = "2"

[workspace.package]
edition = "2021"
version = "18.0.0"
authors = ["MemeSnipe Team"]

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
tracing = "0.1"
redis = { version = "0.25", features = ["tokio-comp"] }
chrono = { version = "0.4", features = ["serde"] }

[profile.release]
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"
opt-level = 3
EOF
    fi
    
    # Create shared directory
    mkdir -p shared
    
    echo -e "${GREEN}✅ Requirements validated${NC}"
}

# Create or update VM
deploy_vm() {
    if gcloud compute instances describe "$VM_NAME" --zone="$ZONE" --quiet &>/dev/null; then
        echo -e "${YELLOW}⚠️ VM exists. Updating...${NC}"
        
        # Stop existing containers
        gcloud compute ssh "$VM_NAME" --zone="$ZONE" --command="cd $REPO_DIR && sudo docker compose down || true" || true
        
        # Clean up
        gcloud compute ssh "$VM_NAME" --zone="$ZONE" --command="sudo rm -rf $REPO_DIR/*"
    else
        echo -e "${GREEN}Creating new VM...${NC}"
        
        gcloud compute instances create "$VM_NAME" \
            --project="$PROJECT_ID" \
            --zone="$ZONE" \
            --machine-type="$MACHINE_TYPE" \
            --boot-disk-size="$DISK_SIZE" \
            --image-family="$IMAGE_FAMILY" \
            --image-project="$IMAGE_PROJECT" \
            --tags=http-server,https-server \
            --metadata=startup-script='#!/bin/bash
                apt-get update
                apt-get install -y apt-transport-https ca-certificates curl gnupg lsb-release git
                
                # Install Docker
                curl -fsSL https://download.docker.com/linux/debian/gpg | gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
                echo "deb [arch=amd64 signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/debian $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null
                apt-get update
                apt-get install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
                
                # Configure Docker
                mkdir -p /etc/docker
                cat > /etc/docker/daemon.json << 'EOF'
{
  "features": {"buildkit": true},
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "10m",
    "max-file": "3"
  }
}
EOF
                systemctl restart docker
                
                # Add user to docker group
                usermod -aG docker ${USER}
            '
        
        echo -e "${YELLOW}Waiting for VM initialization (90s)...${NC}"
        sleep 90
    fi
    
    # Ensure directory exists
    gcloud compute ssh "$VM_NAME" --zone="$ZONE" --command="sudo mkdir -p $REPO_DIR && sudo chown \$USER:\$USER $REPO_DIR"
}

# Deploy application
deploy_app() {
    echo -e "${GREEN}Deploying application...${NC}"
    
    # Create deployment package
    echo -e "${YELLOW}Creating deployment archive...${NC}"
    tar -czf deploy.tar.gz \
        --exclude='.git' \
        --exclude='target' \
        --exclude='*.tar.gz' \
        --exclude='node_modules' \
        .
    
    # Copy to VM
    echo -e "${YELLOW}Copying files to VM...${NC}"
    gcloud compute scp deploy.tar.gz "$VM_NAME":~/deploy.tar.gz --zone="$ZONE"
    
    # Extract and build
    echo -e "${YELLOW}Building services...${NC}"
    gcloud compute ssh "$VM_NAME" --zone="$ZONE" --command="
        cd $REPO_DIR && \
        tar -xzf ~/deploy.tar.gz && \
        rm ~/deploy.tar.gz && \
        export DOCKER_BUILDKIT=1 && \
        export COMPOSE_DOCKER_CLI_BUILD=1 && \
        sudo -E docker compose build --parallel --progress=plain
    "
    
    # Start services
    echo -e "${GREEN}Starting services...${NC}"
    gcloud compute ssh "$VM_NAME" --zone="$ZONE" --command="
        cd $REPO_DIR && \
        sudo docker compose up -d && \
        echo 'Waiting for services to stabilize...' && \
        sleep 10 && \
        sudo docker compose ps
    "
    
    # Clean up local archive
    rm -f deploy.tar.gz
}

# Configure firewall
setup_firewall() {
    FIREWALL_RULE="meme-snipe-v18-access"
    
    if ! gcloud compute firewall-rules describe "$FIREWALL_RULE" --quiet &>/dev/null; then
        echo -e "${YELLOW}Creating firewall rules...${NC}"
        gcloud compute firewall-rules create "$FIREWALL_RULE" \
            --allow=tcp:8080,tcp:9184 \
            --description="MemeSnipe dashboard and metrics" \
            --target-tags=http-server
    fi
}

# Health check
check_health() {
    echo -e "${YELLOW}Checking service health...${NC}"
    
    # Wait for services to start
    sleep 30
    
    # Check Docker status
    gcloud compute ssh "$VM_NAME" --zone="$ZONE" --command="
        cd $REPO_DIR && \
        sudo docker compose ps
    "
    
    # Get external IP
    EXTERNAL_IP=$(gcloud compute instances describe "$VM_NAME" --zone="$ZONE" --format="get(networkInterfaces[0].accessConfigs[0].natIP)")
    
    echo -e "${GREEN}
🎉 DEPLOYMENT COMPLETE!
========================================
📊 Dashboard: http://$EXTERNAL_IP:8080
📈 Metrics: http://$EXTERNAL_IP:9184/metrics
========================================
SSH: gcloud compute ssh $VM_NAME --zone=$ZONE
Logs: gcloud compute ssh $VM_NAME --zone=$ZONE --command='cd $REPO_DIR && sudo docker compose logs -f'
${NC}"
}

# Main execution
main() {
    check_requirements
    deploy_vm
    deploy_app
    setup_firewall
    check_health
}

# Run with error handling
if main; then
    echo -e "${GREEN}✅ Deployment successful!${NC}"
else
    echo -e "${RED}❌ Deployment failed! Check logs above.${NC}"
    exit 1
fi 
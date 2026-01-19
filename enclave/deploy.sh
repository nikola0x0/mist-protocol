#!/bin/bash
# Mist Protocol Enclave Deployment Script
# Usage: ./deploy.sh [rebuild|run|expose|all]

set -e

ENCLAVE_IP="${ENCLAVE_IP:-54.169.193.95}"
KEY_PATH="${KEY_PATH:-~/.ssh/mist-enclave.pem}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log() { echo -e "${GREEN}[+]${NC} $1"; }
warn() { echo -e "${YELLOW}[!]${NC} $1"; }
error() { echo -e "${RED}[x]${NC} $1"; }

# Check if running on EC2 or local
is_ec2() {
    [[ -f /sys/devices/virtual/dmi/id/board_asset_tag ]] && grep -q "i-" /sys/devices/virtual/dmi/id/board_asset_tag 2>/dev/null
}

setup_ec2() {
    log "Setting up EC2 instance..."

    # Install dependencies
    sudo yum install -y aws-nitro-enclaves-cli aws-nitro-enclaves-cli-devel docker git make socat

    # Add user to groups
    sudo usermod -aG ne ec2-user
    sudo usermod -aG docker ec2-user

    # Configure vsock-proxy allowlist
    log "Configuring vsock-proxy allowlist..."
    echo '- {address: fullnode.testnet.sui.io, port: 443}' | sudo tee -a /etc/nitro_enclaves/vsock-proxy.yaml
    echo '- {address: seal-key-server-testnet-1.mystenlabs.com, port: 443}' | sudo tee -a /etc/nitro_enclaves/vsock-proxy.yaml
    echo '- {address: seal-key-server-testnet-2.mystenlabs.com, port: 443}' | sudo tee -a /etc/nitro_enclaves/vsock-proxy.yaml

    # Configure allocator
    log "Configuring enclave allocator (4GB RAM, 2 CPUs)..."
    sudo tee /etc/nitro_enclaves/allocator.yaml > /dev/null <<EOF
---
memory_mib: 4096
cpu_count: 2
EOF

    # Enable services
    sudo systemctl enable --now nitro-enclaves-allocator docker

    log "EC2 setup complete! Please logout and login again for group changes."
}

build_enclave() {
    log "Building enclave image..."
    cd ~/mist-protocol/nautilus
    make ENCLAVE_APP=mist-protocol
    log "Build complete!"
}

run_enclave() {
    log "Starting enclave..."
    cd ~/mist-protocol/nautilus

    # Stop any existing enclave
    sudo nitro-cli terminate-enclave --all 2>/dev/null || true

    # Run in background or foreground based on arg
    if [[ "$1" == "debug" ]]; then
        log "Running in debug mode (logs visible)..."
        make run-debug
    else
        log "Running enclave..."
        make run
        log "Enclave started! Run './deploy.sh expose' in another terminal."
    fi
}

expose_enclave() {
    if [[ -z "$BACKEND_PRIVATE_KEY" ]]; then
        error "BACKEND_PRIVATE_KEY not set!"
        echo "Usage: BACKEND_PRIVATE_KEY='suiprivkey1...' ./deploy.sh expose"
        exit 1
    fi

    log "Exposing enclave ports..."
    cd ~/mist-protocol/nautilus
    chmod +x expose_enclave.sh
    ./expose_enclave.sh
}

transfer_code() {
    log "Transferring code to EC2..."

    # Create tarball (include uncommitted changes)
    tar --exclude='.git' --exclude='node_modules' --exclude='target' --exclude='.next' --exclude='out' -czf /tmp/mist-protocol.tar.gz -C "$(dirname "$0")/.." .

    # Transfer
    scp -i "$KEY_PATH" /tmp/mist-protocol.tar.gz "ec2-user@$ENCLAVE_IP:~"

    log "Transfer complete!"
    log "On EC2, run:"
    echo "  cd ~/mist-protocol && rm -rf * && tar -xzf ~/mist-protocol.tar.gz"
    echo "  cd nautilus && make ENCLAVE_APP=mist-protocol"
}

show_status() {
    log "Checking enclave status..."
    nitro-cli describe-enclaves | jq .
}

show_help() {
    echo "Mist Protocol Enclave Deployment"
    echo ""
    echo "Usage: ./deploy.sh <command>"
    echo ""
    echo "Commands (run on EC2):"
    echo "  setup     - Initial EC2 setup (run once)"
    echo "  build     - Build enclave image"
    echo "  run       - Start enclave"
    echo "  run-debug - Start enclave with console output"
    echo "  expose    - Expose enclave ports (needs BACKEND_PRIVATE_KEY)"
    echo "  status    - Show enclave status"
    echo ""
    echo "Commands (run locally):"
    echo "  transfer  - Transfer code to EC2"
    echo ""
    echo "Environment variables:"
    echo "  ENCLAVE_IP          - EC2 public IP (default: 54.169.193.95)"
    echo "  KEY_PATH            - SSH key path (default: ~/.ssh/mist-enclave.pem)"
    echo "  BACKEND_PRIVATE_KEY - Sui wallet private key (required for expose)"
    echo ""
    echo "Quick deploy workflow:"
    echo "  1. Local:  ./deploy.sh transfer"
    echo "  2. EC2 T1: cd ~/mist-protocol && rm -rf * && tar -xzf ~/mist-protocol.tar.gz && cd nautilus && make ENCLAVE_APP=mist-protocol && ./deploy.sh run"
    echo "  3. EC2 T2: cd ~/mist-protocol/nautilus && BACKEND_PRIVATE_KEY='suiprivkey1...' ./deploy.sh expose"
}

# Main
case "${1:-help}" in
    setup)
        setup_ec2
        ;;
    build)
        build_enclave
        ;;
    run)
        run_enclave
        ;;
    run-debug)
        run_enclave debug
        ;;
    expose)
        expose_enclave
        ;;
    status)
        show_status
        ;;
    transfer)
        transfer_code
        ;;
    *)
        show_help
        ;;
esac

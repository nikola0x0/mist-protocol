#!/bin/bash
# deploy.sh - Deploy Mist Protocol enclave to AWS EC2

set -e

# Configuration
ENCLAVE_APP="mist-protocol"
EC2_USER="ec2-user"
REMOTE_DIR="/home/ec2-user/mist-protocol/nautilus"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check required environment variables
check_env() {
    if [ -z "$EC2_IP" ]; then
        log_error "EC2_IP is not set. Export EC2_IP=<your-instance-ip>"
        exit 1
    fi
    
    if [ -z "$SSH_KEY" ]; then
        SSH_KEY="$HOME/.ssh/mist-protocol.pem"
        log_warn "SSH_KEY not set, using default: $SSH_KEY"
    fi
    
    if [ ! -f "$SSH_KEY" ]; then
        log_error "SSH key not found: $SSH_KEY"
        exit 1
    fi
}

# Build enclave locally
build_local() {
    log_info "Building enclave image locally..."
    
    cd "$(dirname "$0")/../nautilus"
    
    # Clean previous build
    rm -rf out/
    
    # Build
    make ENCLAVE_APP=$ENCLAVE_APP
    
    if [ ! -f "out/nitro.eif" ]; then
        log_error "Build failed: out/nitro.eif not found"
        exit 1
    fi
    
    log_info "Build successful!"
    log_info "PCR values:"
    cat out/nitro.pcrs
    echo ""
}

# Upload to EC2
upload() {
    log_info "Uploading to EC2 ($EC2_IP)..."
    
    # Create remote directory if not exists
    ssh -i "$SSH_KEY" "$EC2_USER@$EC2_IP" "mkdir -p $REMOTE_DIR/out"
    
    # Upload EIF file
    scp -i "$SSH_KEY" out/nitro.eif "$EC2_USER@$EC2_IP:$REMOTE_DIR/out/"
    
    # Upload PCR values
    scp -i "$SSH_KEY" out/nitro.pcrs "$EC2_USER@$EC2_IP:$REMOTE_DIR/out/"
    
    # Upload scripts
    scp -i "$SSH_KEY" expose_enclave.sh "$EC2_USER@$EC2_IP:$REMOTE_DIR/"
    scp -i "$SSH_KEY" Makefile "$EC2_USER@$EC2_IP:$REMOTE_DIR/"
    
    log_info "Upload complete!"
}

# Stop existing enclave
stop_enclave() {
    log_info "Stopping existing enclave..."
    
    ssh -i "$SSH_KEY" "$EC2_USER@$EC2_IP" << 'EOF'
        ENCLAVE_ID=$(nitro-cli describe-enclaves 2>/dev/null | jq -r '.[0].EnclaveID // empty')
        if [ -n "$ENCLAVE_ID" ]; then
            echo "Terminating enclave: $ENCLAVE_ID"
            sudo nitro-cli terminate-enclave --enclave-id "$ENCLAVE_ID"
        else
            echo "No running enclave found"
        fi
EOF
}

# Start enclave
start_enclave() {
    log_info "Starting enclave..."
    
    ssh -i "$SSH_KEY" "$EC2_USER@$EC2_IP" << EOF
        cd $REMOTE_DIR
        sudo nitro-cli run-enclave \
            --cpu-count 2 \
            --memory 4096 \
            --eif-path out/nitro.eif
EOF
    
    log_info "Enclave started!"
}

# Expose HTTP endpoint
expose() {
    log_info "Exposing HTTP endpoint..."
    
    ssh -i "$SSH_KEY" "$EC2_USER@$EC2_IP" << EOF
        cd $REMOTE_DIR
        sh expose_enclave.sh
EOF
}

# Verify deployment
verify() {
    log_info "Verifying deployment..."
    
    sleep 5  # Wait for enclave to initialize
    
    # Health check
    log_info "Running health check..."
    HEALTH=$(curl -s -X GET "http://$EC2_IP:3000/health_check" || echo "FAILED")
    
    if echo "$HEALTH" | grep -q "pk"; then
        log_info "Health check passed!"
        echo "$HEALTH" | jq .
    else
        log_error "Health check failed: $HEALTH"
        exit 1
    fi
    
    # Get attestation
    log_info "Getting attestation..."
    ATTESTATION=$(curl -s -X GET "http://$EC2_IP:3000/get_attestation" || echo "FAILED")
    
    if echo "$ATTESTATION" | grep -q "attestation"; then
        log_info "Attestation retrieved successfully!"
    else
        log_warn "Attestation may have failed (expected in debug mode): $ATTESTATION"
    fi
}

# Main deployment flow
deploy() {
    log_info "=== Mist Protocol Enclave Deployment ==="
    echo ""
    
    check_env
    build_local
    upload
    stop_enclave
    start_enclave
    expose
    verify
    
    echo ""
    log_info "=== Deployment Complete ==="
    log_info "Enclave URL: http://$EC2_IP:3000"
    log_info ""
    log_info "Next steps:"
    log_info "  1. Update PCRs onchain (if code changed)"
    log_info "  2. Register enclave onchain (if new instance)"
    log_info "  3. Share ENCLAVE_OBJECT_ID with team"
}

# Parse arguments
case "${1:-deploy}" in
    deploy)
        deploy
        ;;
    build)
        check_env
        build_local
        ;;
    upload)
        check_env
        upload
        ;;
    stop)
        check_env
        stop_enclave
        ;;
    start)
        check_env
        start_enclave
        expose
        ;;
    verify)
        check_env
        verify
        ;;
    *)
        echo "Usage: $0 {deploy|build|upload|stop|start|verify}"
        echo ""
        echo "Commands:"
        echo "  deploy  - Full deployment (build, upload, restart, verify)"
        echo "  build   - Build enclave locally only"
        echo "  upload  - Upload to EC2 only"
        echo "  stop    - Stop running enclave"
        echo "  start   - Start enclave and expose HTTP"
        echo "  verify  - Verify enclave is running"
        echo ""
        echo "Environment variables:"
        echo "  EC2_IP   - EC2 instance public IP (required)"
        echo "  SSH_KEY  - Path to SSH private key (default: ~/.ssh/mist-protocol.pem)"
        exit 1
        ;;
esac

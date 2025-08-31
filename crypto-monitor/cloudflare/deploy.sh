#!/bin/bash

set -e

# ============================================
# Cloudflare Tunnel Auto Deployment Script
# ============================================

# Configuration
DOMAIN="${DOMAIN:-example.com}"
TUNNEL_NAME="${TUNNEL_NAME:-crypto-monitor}"
EMAIL="${CF_EMAIL}"
API_KEY="${CF_API_KEY}"
ZONE_ID="${CF_ZONE_ID}"

# Paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="$SCRIPT_DIR/config.yml"
CREDENTIALS_DIR="$HOME/.cloudflared"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Functions
print_header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}   Cloudflare Tunnel Deployment${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo ""
}

print_step() {
    echo -e "${YELLOW}► $1${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
    exit 1
}

check_requirements() {
    print_step "Checking requirements..."
    
    # Check cloudflared installation
    if ! command -v cloudflared &> /dev/null; then
        echo "cloudflared not found. Installing..."
        
        if [[ "$OSTYPE" == "darwin"* ]]; then
            brew install cloudflare/cloudflare/cloudflared || print_error "Failed to install cloudflared"
        elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
            # Detect architecture
            ARCH=$(uname -m)
            if [ "$ARCH" = "x86_64" ]; then
                DOWNLOAD_URL="https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64"
            elif [ "$ARCH" = "aarch64" ]; then
                DOWNLOAD_URL="https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-arm64"
            else
                print_error "Unsupported architecture: $ARCH"
            fi
            
            sudo wget -q -O /usr/local/bin/cloudflared "$DOWNLOAD_URL"
            sudo chmod +x /usr/local/bin/cloudflared
        else
            print_error "Unsupported OS: $OSTYPE"
        fi
    fi
    
    print_success "cloudflared is installed"
    
    # Check Docker
    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed"
    fi
    print_success "Docker is installed"
    
    # Check Docker Compose
    if ! command -v docker-compose &> /dev/null; then
        if ! docker compose version &> /dev/null; then
            print_error "Docker Compose is not installed"
        fi
    fi
    print_success "Docker Compose is installed"
}

cloudflare_login() {
    print_step "Checking Cloudflare authentication..."
    
    if ! cloudflared tunnel list &> /dev/null; then
        echo "Please login to Cloudflare..."
        cloudflared tunnel login || print_error "Failed to login to Cloudflare"
    fi
    
    print_success "Authenticated with Cloudflare"
}

create_tunnel() {
    print_step "Setting up tunnel '$TUNNEL_NAME'..."
    
    # Check if tunnel exists
    if cloudflared tunnel list | grep -q "$TUNNEL_NAME"; then
        print_success "Tunnel '$TUNNEL_NAME' already exists"
        TUNNEL_ID=$(cloudflared tunnel list | grep "$TUNNEL_NAME" | awk '{print $1}')
    else
        echo "Creating new tunnel..."
        cloudflared tunnel create "$TUNNEL_NAME" || print_error "Failed to create tunnel"
        TUNNEL_ID=$(cloudflared tunnel list | grep "$TUNNEL_NAME" | awk '{print $1}')
        print_success "Tunnel created with ID: $TUNNEL_ID"
    fi
    
    # Export for use in other functions
    export TUNNEL_ID
}

setup_dns() {
    print_step "Configuring DNS records..."
    
    local subdomains=("monitor" "api.monitor" "ws.monitor" "grafana.monitor" "health.monitor")
    
    for subdomain in "${subdomains[@]}"; do
        echo "  Setting up $subdomain.$DOMAIN..."
        cloudflared tunnel route dns "$TUNNEL_NAME" "$subdomain.$DOMAIN" 2>/dev/null || true
    done
    
    print_success "DNS records configured"
}

generate_config() {
    print_step "Generating configuration files..."
    
    # Create config directory if not exists
    mkdir -p "$SCRIPT_DIR"
    
    # Generate main config
    cat > "$CONFIG_FILE" << EOF
# Auto-generated Cloudflare Tunnel Configuration
# Generated: $(date)
# Domain: $DOMAIN
# Tunnel: $TUNNEL_NAME ($TUNNEL_ID)

tunnel: $TUNNEL_ID
credentials-file: $CREDENTIALS_DIR/$TUNNEL_ID.json

# Logging
loglevel: info
transport-loglevel: warn

# Metrics
metrics: 0.0.0.0:2000

# Origin settings
originRequest:
  keepAliveConnections: 100
  keepAliveTimeout: 90s
  connectTimeout: 30s
  tlsTimeout: 10s
  tcpKeepAlive: 30s
  noHappyEyeballs: false
  disableChunkedEncoding: false
  proxyBufferSize: 65536

# Ingress rules
ingress:
  # API Service
  - hostname: api.monitor.$DOMAIN
    service: http://crypto-monitor:8080
    originRequest:
      noTLSVerify: true
      connectTimeout: 30s
      customRequestHeaders:
        X-Source: "cloudflare-tunnel"
      
  # WebSocket Service
  - hostname: ws.monitor.$DOMAIN
    service: ws://crypto-monitor:8081
    originRequest:
      noTLSVerify: true
      connectTimeout: 0s
      tcpKeepAlive: 30s
      
  # Grafana Dashboard
  - hostname: grafana.monitor.$DOMAIN
    service: http://grafana:3000
    originRequest:
      noTLSVerify: true
      
  # Health Check
  - hostname: health.monitor.$DOMAIN
    service: http://crypto-monitor:8080/health
    originRequest:
      noTLSVerify: true
      connectTimeout: 5s
      
  # Main Web Interface
  - hostname: monitor.$DOMAIN
    service: http://nginx:80
    originRequest:
      noTLSVerify: true
      
  # Catch-all
  - service: http_status:404
EOF
    
    print_success "Configuration generated at $CONFIG_FILE"
    
    # Copy credentials if in Docker context
    if [ -f "$CREDENTIALS_DIR/$TUNNEL_ID.json" ]; then
        mkdir -p "$SCRIPT_DIR/credentials"
        cp "$CREDENTIALS_DIR/$TUNNEL_ID.json" "$SCRIPT_DIR/credentials/"
        print_success "Credentials copied to $SCRIPT_DIR/credentials/"
    fi
}

validate_config() {
    print_step "Validating configuration..."
    
    cloudflared tunnel --config "$CONFIG_FILE" ingress validate || print_error "Invalid configuration"
    
    print_success "Configuration is valid"
}

create_systemd_service() {
    if [[ "$OSTYPE" != "linux-gnu"* ]]; then
        return
    fi
    
    print_step "Creating systemd service..."
    
    sudo tee /etc/systemd/system/cloudflared-tunnel.service > /dev/null << EOF
[Unit]
Description=Cloudflare Tunnel for Crypto Monitor
After=network.target

[Service]
Type=simple
User=$USER
ExecStart=/usr/local/bin/cloudflared tunnel --config $CONFIG_FILE run
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF
    
    sudo systemctl daemon-reload
    sudo systemctl enable cloudflared-tunnel.service
    
    print_success "Systemd service created"
}

start_services() {
    print_step "Starting services..."
    
    # Start main services
    echo "Starting Docker services..."
    cd "$SCRIPT_DIR/.."
    docker-compose up -d
    
    # Start tunnel
    echo "Starting Cloudflare tunnel..."
    docker-compose -f docker-compose.yml -f docker-compose.cloudflare.yml up -d
    
    print_success "All services started"
}

test_connection() {
    print_step "Testing connections..."
    
    echo "Waiting for services to be ready..."
    sleep 10
    
    # Test endpoints
    local endpoints=(
        "https://health.monitor.$DOMAIN"
        "https://api.monitor.$DOMAIN/health"
        "https://monitor.$DOMAIN"
    )
    
    for endpoint in "${endpoints[@]}"; do
        echo -n "  Testing $endpoint... "
        if curl -s -o /dev/null -w "%{http_code}" "$endpoint" | grep -q "200\|301\|302"; then
            echo -e "${GREEN}OK${NC}"
        else
            echo -e "${RED}FAILED${NC}"
        fi
    done
}

print_summary() {
    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}   Deployment Complete!${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "Access URLs:"
    echo -e "  Main Site:    ${BLUE}https://monitor.$DOMAIN${NC}"
    echo -e "  API:          ${BLUE}https://api.monitor.$DOMAIN${NC}"
    echo -e "  WebSocket:    ${BLUE}wss://ws.monitor.$DOMAIN${NC}"
    echo -e "  Grafana:      ${BLUE}https://grafana.monitor.$DOMAIN${NC}"
    echo -e "  Health:       ${BLUE}https://health.monitor.$DOMAIN${NC}"
    echo ""
    echo "Management Commands:"
    echo "  View logs:    docker-compose logs -f cloudflared"
    echo "  Stop tunnel:  docker-compose stop cloudflared"
    echo "  Start tunnel: docker-compose start cloudflared"
    echo "  Status:       docker-compose ps"
    echo ""
    echo "Tunnel Information:"
    echo "  Tunnel Name:  $TUNNEL_NAME"
    echo "  Tunnel ID:    $TUNNEL_ID"
    echo "  Config:       $CONFIG_FILE"
    echo ""
}

# Main execution
main() {
    print_header
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --domain)
                DOMAIN="$2"
                shift 2
                ;;
            --tunnel-name)
                TUNNEL_NAME="$2"
                shift 2
                ;;
            --skip-dns)
                SKIP_DNS=true
                shift
                ;;
            --skip-start)
                SKIP_START=true
                shift
                ;;
            *)
                echo "Unknown option: $1"
                echo "Usage: $0 [--domain example.com] [--tunnel-name name] [--skip-dns] [--skip-start]"
                exit 1
                ;;
        esac
    done
    
    # Execute deployment steps
    check_requirements
    cloudflare_login
    create_tunnel
    
    if [ "$SKIP_DNS" != "true" ]; then
        setup_dns
    fi
    
    generate_config
    validate_config
    
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        create_systemd_service
    fi
    
    if [ "$SKIP_START" != "true" ]; then
        start_services
        test_connection
    fi
    
    print_summary
}

# Run main function
main "$@"
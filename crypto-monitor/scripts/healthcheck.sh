#!/bin/bash

# Health check script for crypto-monitor

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
API_URL="${API_URL:-http://localhost:8080}"
MAX_RETRIES="${MAX_RETRIES:-3}"
RETRY_DELAY="${RETRY_DELAY:-5}"

# Function to check service health
check_service() {
    local service_name=$1
    local check_command=$2
    
    echo -n "Checking $service_name... "
    
    if eval "$check_command" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC}"
        return 0
    else
        echo -e "${RED}✗${NC}"
        return 1
    fi
}

# Function to check with retries
check_with_retry() {
    local service_name=$1
    local check_command=$2
    local retries=0
    
    while [ $retries -lt $MAX_RETRIES ]; do
        if check_service "$service_name" "$check_command"; then
            return 0
        fi
        
        retries=$((retries + 1))
        if [ $retries -lt $MAX_RETRIES ]; then
            echo "  Retrying in ${RETRY_DELAY} seconds... (Attempt $((retries + 1))/$MAX_RETRIES)"
            sleep $RETRY_DELAY
        fi
    done
    
    return 1
}

# Main health checks
echo "======================================"
echo "Crypto Monitor Health Check"
echo "======================================"
echo ""

# Track overall health
HEALTHY=true

# Check API endpoint
if ! check_with_retry "API Server" "curl -sf ${API_URL}/health"; then
    HEALTHY=false
fi

# Check WebSocket endpoint
if ! check_with_retry "WebSocket Server" "curl -sf ${API_URL}/ws"; then
    HEALTHY=false
fi

# Check database connection
if ! check_with_retry "PostgreSQL" "pg_isready -h ${DB_HOST:-localhost} -p ${DB_PORT:-5432}"; then
    HEALTHY=false
fi

# Check Redis
if ! check_with_retry "Redis" "redis-cli -h ${REDIS_HOST:-localhost} ping"; then
    HEALTHY=false
fi

# Check Fluvio
if ! check_with_retry "Fluvio" "fluvio cluster status"; then
    echo -e "  ${YELLOW}Warning: Fluvio check failed (non-critical)${NC}"
fi

echo ""
echo "======================================"

if [ "$HEALTHY" = true ]; then
    echo -e "${GREEN}All critical services are healthy!${NC}"
    exit 0
else
    echo -e "${RED}Some services are unhealthy!${NC}"
    exit 1
fi
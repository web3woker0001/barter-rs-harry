#!/bin/bash

# Docker build script that handles barter dependencies

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}Preparing Docker build...${NC}"

# Create temporary build directory
BUILD_DIR="docker-build-context"
rm -rf $BUILD_DIR
mkdir -p $BUILD_DIR

# Copy crypto-monitor files
echo "Copying crypto-monitor files..."
cp -r monitor-* $BUILD_DIR/
cp Cargo.toml $BUILD_DIR/
cp -r migrations $BUILD_DIR/ 2>/dev/null || true
cp config.example.yaml $BUILD_DIR/ 2>/dev/null || true

# Copy barter dependencies from parent directory
echo "Copying barter dependencies..."
if [ -d "../barter" ]; then
    cp -r ../barter $BUILD_DIR/
    cp -r ../barter-data $BUILD_DIR/
    cp -r ../barter-execution $BUILD_DIR/
    cp -r ../barter-integration $BUILD_DIR/
    cp -r ../barter-instrument $BUILD_DIR/
    echo -e "${GREEN}Barter dependencies copied${NC}"
else
    echo -e "${RED}Warning: Barter dependencies not found in parent directory${NC}"
    echo "Creating stub packages..."
    
    # Create stub barter packages for testing
    for pkg in barter barter-data barter-execution barter-integration barter-instrument; do
        mkdir -p $BUILD_DIR/$pkg/src
        cat > $BUILD_DIR/$pkg/Cargo.toml << EOF
[package]
name = "$pkg"
version = "0.1.0"
edition = "2021"

[dependencies]
EOF
        echo "// Stub implementation" > $BUILD_DIR/$pkg/src/lib.rs
    done
fi

# Copy Dockerfile
cp Dockerfile.alpine $BUILD_DIR/Dockerfile

# Create updated Dockerfile with correct paths
cat > $BUILD_DIR/Dockerfile << 'EOF'
# Build stage
FROM rust:1.82-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    protobuf-dev \
    cmake \
    make \
    g++

# Create app directory
WORKDIR /usr/src/app

# Copy all files
COPY . .

# Build the application
RUN cargo build --release --package monitor-app || \
    cargo build --release --bin crypto-monitor

# Runtime stage
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    openssl \
    libgcc \
    curl \
    postgresql-client

# Create non-root user
RUN adduser -D -u 1000 monitor && \
    mkdir -p /app/data /app/logs && \
    chown -R monitor:monitor /app

WORKDIR /app

# Copy binary from builder
COPY --from=builder /usr/src/app/target/release/crypto-monitor /app/ 2>/dev/null || \
    COPY --from=builder /usr/src/app/target/release/monitor-app /app/crypto-monitor

# Copy configuration
COPY config.example.yaml /app/config.example.yaml

# Create health check script
RUN echo '#!/bin/sh\ncurl -f http://localhost:8080/health || exit 1' > /app/healthcheck.sh && \
    chmod +x /app/healthcheck.sh

# Switch to non-root user
USER monitor

# Expose ports
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/app/healthcheck.sh"]

# Set default environment variables
ENV RUST_LOG=info
ENV CONFIG_PATH=/app/config/config.yaml

# Run the application
ENTRYPOINT ["/app/crypto-monitor"]
CMD ["--config", "/app/config/config.yaml"]
EOF

# Build Docker image
echo -e "${YELLOW}Building Docker image...${NC}"
cd $BUILD_DIR
docker build -t crypto-monitor:latest . "$@"
BUILD_RESULT=$?

# Clean up
cd ..
rm -rf $BUILD_DIR

if [ $BUILD_RESULT -eq 0 ]; then
    echo -e "${GREEN}Docker build successful!${NC}"
    echo ""
    echo "Image created: crypto-monitor:latest"
    echo ""
    echo "To run:"
    echo "  docker run -p 8080:8080 crypto-monitor:latest"
    echo ""
    echo "With docker-compose:"
    echo "  docker-compose up -d"
else
    echo -e "${RED}Docker build failed!${NC}"
    exit 1
fi
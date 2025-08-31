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

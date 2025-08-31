#!/bin/bash

set -e

# Wait for dependencies
echo "Waiting for dependencies to be ready..."

# Wait for PostgreSQL
until pg_isready -h "${DB_HOST:-postgres}" -p "${DB_PORT:-5432}"; do
    echo "Waiting for PostgreSQL..."
    sleep 2
done
echo "PostgreSQL is ready!"

# Wait for Fluvio
MAX_RETRIES=30
RETRY_COUNT=0
until fluvio cluster status > /dev/null 2>&1 || [ $RETRY_COUNT -eq $MAX_RETRIES ]; do
    echo "Waiting for Fluvio... (Attempt $((RETRY_COUNT + 1))/$MAX_RETRIES)"
    sleep 5
    RETRY_COUNT=$((RETRY_COUNT + 1))
done

if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
    echo "Warning: Fluvio is not ready after $MAX_RETRIES attempts. Continuing anyway..."
else
    echo "Fluvio is ready!"
fi

# Run database migrations
if [ "${RUN_MIGRATIONS:-true}" = "true" ]; then
    echo "Running database migrations..."
    sqlx migrate run
    echo "Migrations completed!"
fi

# Create Fluvio topics if they don't exist
if command -v fluvio &> /dev/null; then
    echo "Creating Fluvio topics..."
    
    TOPICS=(
        "crypto-monitor.market.trades"
        "crypto-monitor.market.orderbook"
        "crypto-monitor.market.candles"
        "crypto-monitor.anomalies"
        "crypto-monitor.alerts"
        "crypto-monitor.trades"
    )
    
    for topic in "${TOPICS[@]}"; do
        fluvio topic create "$topic" --partitions 3 --replication 1 2>/dev/null || echo "Topic $topic already exists"
    done
    
    echo "Fluvio topics ready!"
fi

# Start the application
echo "Starting Crypto Monitor..."
exec "$@"
#!/bin/bash

# Script to fix Docker build issues

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}Fixing Docker build issues...${NC}"

# Option 1: Use the alternative Alpine Dockerfile
use_alpine_build() {
    echo -e "${GREEN}Using Alpine-based Dockerfile...${NC}"
    
    # Backup original Dockerfile
    cp Dockerfile Dockerfile.backup
    
    # Use Alpine version
    cp Dockerfile.alpine Dockerfile
    
    echo "Building with Alpine Dockerfile..."
    docker build -t crypto-monitor:alpine .
    
    echo -e "${GREEN}Build completed with Alpine!${NC}"
}

# Option 2: Use pre-built sqlx binary
use_prebuilt_sqlx() {
    echo -e "${GREEN}Using pre-built sqlx binary...${NC}"
    
    # Update migrate Dockerfile
    cp Dockerfile.migrate.simple Dockerfile.migrate
    
    echo "Updated migration Dockerfile to use pre-built binary"
}

# Option 3: Build without sqlx-cli (manual migrations)
build_without_sqlx() {
    echo -e "${GREEN}Building without sqlx-cli...${NC}"
    
    cat > Dockerfile.migrate.manual << 'EOF'
FROM postgres:16-alpine

# Copy migrations
COPY migrations /migrations

# Create entrypoint
COPY <<'SCRIPT' /docker-entrypoint-initdb.d/01-migrations.sh
#!/bin/bash
for file in /migrations/*.sql; do
    echo "Running migration: $file"
    psql -U $POSTGRES_USER -d $POSTGRES_DB -f "$file"
done
SCRIPT

RUN chmod +x /docker-entrypoint-initdb.d/01-migrations.sh
EOF

    echo "Created manual migration Dockerfile"
}

# Option 4: Fix Cargo dependencies
fix_cargo_deps() {
    echo -e "${GREEN}Fixing Cargo dependencies...${NC}"
    
    # Update Cargo.toml to use compatible versions
    cat > monitor-app/Cargo.toml.fixed << 'EOF'
[package]
name = "monitor-app"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[[bin]]
name = "crypto-monitor"
path = "src/main.rs"

[dependencies]
# Use specific versions to avoid conflicts
sqlx = { version = "=0.7.4", features = ["runtime-tokio-native-tls", "postgres", "chrono", "uuid"] }

# Rest of dependencies...
monitor-core = { path = "../monitor-core" }
monitor-anomaly = { path = "../monitor-anomaly" }
monitor-api = { path = "../monitor-api" }
monitor-notifier = { path = "../monitor-notifier" }
monitor-trader = { path = "../monitor-trader" }
monitor-config = { path = "../monitor-config" }

barter = { workspace = true }
barter-data = { workspace = true }
barter-execution = { workspace = true }
barter-instrument = { workspace = true }

fluvio = { workspace = true }
tokio = { workspace = true }
futures = { workspace = true }
async-trait = { workspace = true }

serde = { workspace = true }
serde_json = { workspace = true }
serde_yaml = { workspace = true }

tracing = { workspace = true }
tracing-subscriber = { workspace = true }

config = { workspace = true }
clap = { version = "4.5", features = ["derive", "env"] }

chrono = { workspace = true }
anyhow = { workspace = true }

ctrlc = "3.4"
EOF

    echo "Created fixed Cargo.toml"
}

# Main menu
echo ""
echo "Choose a fix option:"
echo "1) Use Alpine-based Dockerfile (recommended)"
echo "2) Use pre-built sqlx binary for migrations"
echo "3) Build without sqlx-cli (manual migrations)"
echo "4) Fix Cargo dependencies"
echo "5) Apply all fixes"
echo ""
read -p "Enter option (1-5): " option

case $option in
    1)
        use_alpine_build
        ;;
    2)
        use_prebuilt_sqlx
        ;;
    3)
        build_without_sqlx
        ;;
    4)
        fix_cargo_deps
        ;;
    5)
        use_alpine_build
        use_prebuilt_sqlx
        fix_cargo_deps
        echo -e "${GREEN}All fixes applied!${NC}"
        ;;
    *)
        echo -e "${RED}Invalid option${NC}"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}Fix completed!${NC}"
echo ""
echo "Now you can build with:"
echo "  docker-compose build"
echo "or"
echo "  docker build -t crypto-monitor ."
echo ""
echo "To use the Alpine build directly:"
echo "  docker build -f Dockerfile.alpine -t crypto-monitor ."
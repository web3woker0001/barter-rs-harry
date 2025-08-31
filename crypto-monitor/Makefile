.PHONY: help build up down logs clean test dev prod health

# Default target
help:
	@echo "Crypto Monitor - Make Commands"
	@echo ""
	@echo "Development:"
	@echo "  make dev          - Start development environment with hot reload"
	@echo "  make build-dev    - Build development Docker images"
	@echo ""
	@echo "Production:"
	@echo "  make prod         - Start production environment"
	@echo "  make build        - Build production Docker images"
	@echo ""
	@echo "Common:"
	@echo "  make up           - Start all services"
	@echo "  make down         - Stop all services"
	@echo "  make logs         - View logs from all services"
	@echo "  make health       - Check health of all services"
	@echo "  make clean        - Clean up containers, volumes, and images"
	@echo ""
	@echo "Database:"
	@echo "  make migrate      - Run database migrations"
	@echo "  make db-shell     - Open PostgreSQL shell"
	@echo ""
	@echo "Testing:"
	@echo "  make test         - Run all tests"
	@echo "  make test-unit    - Run unit tests"
	@echo "  make test-integration - Run integration tests"

# Environment setup
.env:
	cp .env.example .env
	@echo "Created .env file. Please update it with your configuration."

# Development commands
dev: .env
	docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d
	@echo "Development environment started!"
	@echo "API: http://localhost:8080"
	@echo "WebSocket: ws://localhost:8081/ws"
	@echo "Grafana: http://localhost:3000 (admin/admin)"

build-dev:
	docker-compose -f docker-compose.yml -f docker-compose.dev.yml build

# Production commands
prod: .env
	docker-compose up -d
	@echo "Production environment started!"

build:
	docker-compose build

# Common commands
up: .env
	docker-compose up -d

down:
	docker-compose down

logs:
	docker-compose logs -f

health:
	@./scripts/healthcheck.sh

clean:
	docker-compose down -v
	docker system prune -f

# Database commands
migrate:
	docker-compose run --rm migrate migrate run

db-shell:
	docker-compose exec postgres psql -U crypto_monitor -d crypto_monitor

db-backup:
	@mkdir -p backups
	docker-compose exec postgres pg_dump -U crypto_monitor crypto_monitor | gzip > backups/backup_$$(date +%Y%m%d_%H%M%S).sql.gz
	@echo "Database backed up to backups/"

db-restore:
	@echo "Usage: make db-restore FILE=backups/backup_YYYYMMDD_HHMMSS.sql.gz"
	@[ -f "$(FILE)" ] || (echo "File not found: $(FILE)" && exit 1)
	gunzip -c $(FILE) | docker-compose exec -T postgres psql -U crypto_monitor crypto_monitor

# Testing commands
test:
	cargo test --all

test-unit:
	cargo test --lib --all

test-integration:
	cargo test --test '*' --all

# Monitoring commands
monitor-start:
	docker-compose up -d prometheus grafana

monitor-stop:
	docker-compose stop prometheus grafana

# Fluvio commands
fluvio-topics:
	docker-compose exec fluvio fluvio topic list

fluvio-consume:
	@echo "Usage: make fluvio-consume TOPIC=crypto-monitor.market.trades"
	docker-compose exec fluvio fluvio consume $(TOPIC) --from-beginning

# Build and push to registry
docker-push:
	docker build -t crypto-monitor:latest .
	docker tag crypto-monitor:latest $(REGISTRY)/crypto-monitor:latest
	docker push $(REGISTRY)/crypto-monitor:latest

# Local development without Docker
local-run:
	cargo run --bin crypto-monitor -- --config config.yaml

local-watch:
	cargo watch -x 'run --bin crypto-monitor -- --config config.yaml'

# Linting and formatting
fmt:
	cargo fmt --all

lint:
	cargo clippy --all -- -D warnings

check:
	cargo check --all

# Documentation
docs:
	cargo doc --no-deps --open

# Release builds
release:
	cargo build --release --all

# Performance profiling
perf:
	cargo build --release
	perf record --call-graph=dwarf ./target/release/crypto-monitor --config config.yaml
	perf report

# Cloudflare Tunnel commands
cloudflare-setup:
	@echo "Setting up Cloudflare Tunnel..."
	./cloudflare/deploy.sh --domain $(DOMAIN)

cloudflare-up:
	docker-compose -f docker-compose.yml -f docker-compose.cloudflare.yml up -d

cloudflare-down:
	docker-compose -f docker-compose.yml -f docker-compose.cloudflare.yml down

cloudflare-logs:
	docker-compose logs -f cloudflared

cloudflare-status:
	@echo "Checking Cloudflare Tunnel status..."
	@docker-compose ps cloudflared
	@echo ""
	@echo "Tunnel metrics:"
	@curl -s http://localhost:2000/metrics | grep cloudflared || echo "Metrics not available"

cloudflare-restart:
	docker-compose restart cloudflared

cloudflare-validate:
	cloudflared tunnel --config cloudflare/config.yml ingress validate
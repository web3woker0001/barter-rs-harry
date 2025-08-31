#!/bin/bash

# API Testing Script for Crypto Monitor
# This script contains all curl commands to test the API endpoints

# Configuration
API_BASE="${API_BASE:-http://localhost:8080}"
WS_BASE="${WS_BASE:-ws://localhost:8081}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "=========================================="
echo "Crypto Monitor API Test Suite"
echo "API Base: $API_BASE"
echo "=========================================="
echo ""

# Function to print section header
print_section() {
    echo -e "${BLUE}=== $1 ===${NC}"
}

# Function to print test name
print_test() {
    echo -e "${YELLOW}► $1${NC}"
}

# ============================================
# Health & Status Endpoints
# ============================================

print_section "Health & Status Endpoints"

print_test "Health Check"
curl -X GET "$API_BASE/health" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "System Status"
curl -X GET "$API_BASE/api/v1/status" \
  -H "Content-Type: application/json" | jq '.'
echo ""

# ============================================
# Market Data Endpoints
# ============================================

print_section "Market Data Endpoints"

print_test "Get Market Stats (All)"
curl -X GET "$API_BASE/api/v1/market/stats" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Get Market Stats (Filtered by Symbol)"
curl -X GET "$API_BASE/api/v1/market/stats?symbol=BTC/USDT" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Get Market Stats (Filtered by Exchange)"
curl -X GET "$API_BASE/api/v1/market/stats?exchange=binance" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Get Market Stats (Multiple Filters)"
curl -X GET "$API_BASE/api/v1/market/stats?symbol=BTC/USDT&exchange=binance" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Get Market History"
curl -X GET "$API_BASE/api/v1/market/history?symbol=BTC/USDT&limit=10" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Get Market History with Time Range"
curl -X GET "$API_BASE/api/v1/market/history?symbol=BTC/USDT&from=2024-01-01T00:00:00Z&to=2024-01-02T00:00:00Z" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Get Order Book"
curl -X GET "$API_BASE/api/v1/market/orderbook?symbol=BTC/USDT&exchange=binance" \
  -H "Content-Type: application/json" | jq '.'
echo ""

# ============================================
# Anomaly Detection Endpoints
# ============================================

print_section "Anomaly Detection Endpoints"

print_test "Get All Anomalies"
curl -X GET "$API_BASE/api/v1/anomalies" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Get Anomalies (Filtered by Type)"
curl -X GET "$API_BASE/api/v1/anomalies?anomaly_type=VolumeSpike" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Get Anomalies (Filtered by Severity)"
curl -X GET "$API_BASE/api/v1/anomalies?severity=High" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Get Anomalies (With Time Range)"
curl -X GET "$API_BASE/api/v1/anomalies?from=2024-01-01T00:00:00Z&to=2024-01-02T00:00:00Z&limit=20" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Get Anomaly Statistics"
curl -X GET "$API_BASE/api/v1/anomalies/stats" \
  -H "Content-Type: application/json" | jq '.'
echo ""

# ============================================
# Trading Configuration Endpoints
# ============================================

print_section "Trading Configuration Endpoints"

print_test "Get Trading Configuration"
curl -X GET "$API_BASE/api/v1/trading/config" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Update Trading Configuration"
curl -X POST "$API_BASE/api/v1/trading/config" \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true,
    "symbol": "BTC/USDT",
    "exchange": "binance",
    "max_position_size": 1000.0,
    "stop_loss_percentage": 3.0,
    "take_profit_percentage": 6.0
  }' | jq '.'
echo ""

print_test "Disable Auto Trading"
curl -X POST "$API_BASE/api/v1/trading/config" \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": false,
    "symbol": "BTC/USDT",
    "exchange": "binance",
    "max_position_size": 0,
    "stop_loss_percentage": 0,
    "take_profit_percentage": 0
  }' | jq '.'
echo ""

# ============================================
# Position Management Endpoints
# ============================================

print_section "Position Management Endpoints"

print_test "Get All Positions"
curl -X GET "$API_BASE/api/v1/trading/positions" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Get Open Positions"
curl -X GET "$API_BASE/api/v1/trading/positions?status=open" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Get Closed Positions"
curl -X GET "$API_BASE/api/v1/trading/positions?status=closed" \
  -H "Content-Type: application/json" | jq '.'
echo ""

# ============================================
# Order Management Endpoints
# ============================================

print_section "Order Management Endpoints"

print_test "Get All Orders"
curl -X GET "$API_BASE/api/v1/trading/orders" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Get Orders by Status"
curl -X GET "$API_BASE/api/v1/trading/orders?status=filled" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Place Market Order (Buy)"
curl -X POST "$API_BASE/api/v1/trading/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "BTC/USDT",
    "exchange": "binance",
    "side": "buy",
    "type": "market",
    "quantity": 0.001
  }' | jq '.'
echo ""

print_test "Place Limit Order (Sell)"
curl -X POST "$API_BASE/api/v1/trading/orders" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "BTC/USDT",
    "exchange": "binance",
    "side": "sell",
    "type": "limit",
    "quantity": 0.001,
    "price": 50000.0
  }' | jq '.'
echo ""

print_test "Cancel Order"
# Replace ORDER_ID with actual order ID
curl -X DELETE "$API_BASE/api/v1/trading/orders/ORDER_ID" \
  -H "Content-Type: application/json" | jq '.'
echo ""

# ============================================
# Alert Configuration Endpoints
# ============================================

print_section "Alert Configuration Endpoints"

print_test "Get Alert Configuration"
curl -X GET "$API_BASE/api/v1/alerts/config" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Update Alert Configuration (Enable Telegram)"
curl -X POST "$API_BASE/api/v1/alerts/config" \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true,
    "channels": [
      {
        "channel_type": "Telegram",
        "config": {
          "chat_id": "-123456789",
          "send_images": true
        }
      }
    ],
    "severity_threshold": "Medium"
  }' | jq '.'
echo ""

print_test "Update Alert Configuration (Multiple Channels)"
curl -X POST "$API_BASE/api/v1/alerts/config" \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true,
    "channels": [
      {
        "channel_type": "Telegram",
        "config": {
          "chat_id": "-123456789"
        }
      },
      {
        "channel_type": "Email",
        "config": {
          "to_addresses": ["alert@example.com"]
        }
      }
    ],
    "severity_threshold": "High"
  }' | jq '.'
echo ""

print_test "Get Alert History"
curl -X GET "$API_BASE/api/v1/alerts/history" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Get Alert History (With Filters)"
curl -X GET "$API_BASE/api/v1/alerts/history?severity=Critical&limit=10" \
  -H "Content-Type: application/json" | jq '.'
echo ""

# ============================================
# WebSocket Connection Test
# ============================================

print_section "WebSocket Connection Test"

print_test "WebSocket Connection Test (requires wscat)"
echo "Install wscat: npm install -g wscat"
echo "Connect: wscat -c $WS_BASE/ws"
echo ""
echo "Subscribe to all events:"
echo '{"msg_type":"Subscribe","data":{"channel":"all","symbols":[],"exchanges":[]}}'
echo ""
echo "Subscribe to specific symbol:"
echo '{"msg_type":"Subscribe","data":{"channel":"market","symbols":["BTC/USDT"],"exchanges":["binance"]}}'
echo ""
echo "Subscribe to anomalies only:"
echo '{"msg_type":"Subscribe","data":{"channel":"anomalies","symbols":[],"exchanges":[]}}'
echo ""
echo "Send heartbeat:"
echo '{"msg_type":"Heartbeat","data":{}}'
echo ""

# ============================================
# Metrics Endpoint (Prometheus)
# ============================================

print_section "Metrics Endpoint"

print_test "Get Prometheus Metrics"
curl -X GET "$API_BASE/metrics" \
  -H "Content-Type: application/json"
echo ""

# ============================================
# Advanced Query Examples
# ============================================

print_section "Advanced Query Examples"

print_test "Complex Market Query"
curl -X GET "$API_BASE/api/v1/market/history?symbol=ETH/USDT&exchange=binance&from=2024-01-01T00:00:00Z&to=2024-01-01T01:00:00Z&limit=100" \
  -H "Content-Type: application/json" | jq '.'
echo ""

print_test "Complex Anomaly Query"
curl -X GET "$API_BASE/api/v1/anomalies?symbol=BTC/USDT&exchange=binance&anomaly_type=PriceSpike&severity=High&from=2024-01-01T00:00:00Z&limit=50" \
  -H "Content-Type: application/json" | jq '.'
echo ""

# ============================================
# Batch Operations
# ============================================

print_section "Batch Operations Examples"

print_test "Batch Order Placement"
curl -X POST "$API_BASE/api/v1/trading/orders/batch" \
  -H "Content-Type: application/json" \
  -d '{
    "orders": [
      {
        "symbol": "BTC/USDT",
        "exchange": "binance",
        "side": "buy",
        "type": "limit",
        "quantity": 0.001,
        "price": 40000.0
      },
      {
        "symbol": "ETH/USDT",
        "exchange": "binance",
        "side": "buy",
        "type": "limit",
        "quantity": 0.01,
        "price": 2500.0
      }
    ]
  }' | jq '.'
echo ""

# ============================================
# Error Testing
# ============================================

print_section "Error Response Testing"

print_test "Invalid Endpoint (404)"
curl -X GET "$API_BASE/api/v1/invalid" \
  -H "Content-Type: application/json" -w "\nHTTP Status: %{http_code}\n"
echo ""

print_test "Invalid Request Body (400)"
curl -X POST "$API_BASE/api/v1/trading/orders" \
  -H "Content-Type: application/json" \
  -d '{"invalid": "data"}' -w "\nHTTP Status: %{http_code}\n" | jq '.'
echo ""

print_test "Method Not Allowed (405)"
curl -X DELETE "$API_BASE/api/v1/market/stats" \
  -H "Content-Type: application/json" -w "\nHTTP Status: %{http_code}\n"
echo ""

echo ""
echo "=========================================="
echo "API Test Suite Completed"
echo "=========================================="
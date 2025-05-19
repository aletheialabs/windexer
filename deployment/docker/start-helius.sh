#!/bin/bash
set -e
# Script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Load environment variables
if [ -f .env ]; then
  source .env
  echo "Loaded environment variables from .env"
else
  echo "Warning: .env file not found. Using default or existing environment variables."
fi

# Ensure API key is provided
if [ -z "$HELIUS_API_KEY" ]; then
  echo "Error: HELIUS_API_KEY is not set. Please set it in the .env file."
  exit 1
fi

# Configure RPC URLs if not already done
if [ -z "$SOLANA_RPC_URL" ]; then
  export SOLANA_RPC_URL="https://mainnet.helius-rpc.com/?api-key=${HELIUS_API_KEY}"
  echo "Set SOLANA_RPC_URL to Helius mainnet"
fi

if [ -z "$SOLANA_WS_URL" ]; then
  export SOLANA_WS_URL="wss://mainnet.helius-rpc.com/?api-key=${HELIUS_API_KEY}"
  echo "Set SOLANA_WS_URL to Helius mainnet WebSocket"
fi

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Starting wIndexer Helius Environment ===${NC}"
echo -e "${YELLOW}Configuration:${NC}"
echo -e "  Network: ${NETWORK:-mainnet}"
echo -e "  API port: ${API_PORT:-3000}"
echo -e "  Log level: ${RUST_LOG:-info}"
echo -e "  RPC URL: ${SOLANA_RPC_URL}"
echo -e "  WS URL: ${SOLANA_WS_URL}"

# Clean up any existing containers
echo -e "${YELLOW}Cleaning up existing containers...${NC}"
docker-compose -f docker-compose.helius.yml down --remove-orphans 2>/dev/null || true

# Ensure data directories exist
mkdir -p ${STORAGE_DIR:-/home/winuser/windexer/data}/{postgres}

# Build and start the API and postgres containers only
echo -e "${YELLOW}Building and starting services...${NC}"
docker-compose -f docker-compose.helius.yml build
docker-compose -f docker-compose.helius.yml up -d

echo -e "${GREEN}Services started!${NC}"
echo -e "${YELLOW}API endpoint:${NC}"
echo -e "  http://localhost:${API_PORT:-3000}/api/status"
echo -e "  http://localhost:${API_PORT:-3000}/api/health"
echo -e "  http://localhost:${API_PORT:-3000}/api/blocks/latest"

echo -e "${GREEN}You can now use the Jito MEV analyzer with these endpoints:${NC}"
echo -e "  cd ~/windexer/examples/python"
echo -e "  ./run_jito_analyzer.sh --api-url http://localhost:${API_PORT:-3000}/api --blocks 10 --cli-only"

echo -e "${YELLOW}To stop the environment:${NC}"
echo -e "  docker-compose -f docker-compose.helius.yml down"

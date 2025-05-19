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
echo -e "  Nodes: ${NODES:-3}"
echo -e "  Indexers: ${INDEXERS:-2}"
echo -e "  Base port: ${BASE_PORT:-9000}"
echo -e "  Indexer base port: ${INDEXER_BASE_PORT:-10000}"
echo -e "  API port: ${API_PORT:-3000}"
echo -e "  Log level: ${RUST_LOG:-info}"
echo -e "  RPC URL: ${SOLANA_RPC_URL}"
echo -e "  WS URL: ${SOLANA_WS_URL}"

# Ensure data directories exist
mkdir -p ${STORAGE_DIR:-/home/winuser/windexer/data}/{node_0,postgres}
for i in $(seq 1 $((${NODES:-3}-1))); do
  mkdir -p ${STORAGE_DIR:-/home/winuser/windexer/data}/node_$i
done
for i in $(seq 1 ${INDEXERS:-2}); do
  mkdir -p ${STORAGE_DIR:-/home/winuser/windexer/data}/indexer_$i
done

# Generate docker-compose.override.yml
echo -e "${YELLOW}Generating docker-compose configuration...${NC}"
chmod +x ./generate-helius-compose.sh
./generate-helius-compose.sh

# Build and start the containers
echo -e "${YELLOW}Building and starting services...${NC}"
docker-compose -f docker-compose.helius.yml -f docker-compose.helius.override.yml build
docker-compose -f docker-compose.helius.yml -f docker-compose.helius.override.yml up -d

echo -e "${GREEN}Services started!${NC}"
echo -e "${YELLOW}Services:${NC}"

# Display node info
for i in $(seq 0 $((${NODES:-3}-1))); do
  NODE_PORT=$((${BASE_PORT:-9000} + i*100))
  echo -e "  Node $i: http://localhost:$NODE_PORT/api/status"
done

# Display indexer info
for i in $(seq 1 ${INDEXERS:-2}); do
  INDEXER_PORT=$((${INDEXER_BASE_PORT:-10000} + i))
  echo -e "  Indexer $i: http://localhost:${INDEXER_PORT}/api/status"
done

echo -e "${YELLOW}API endpoint:${NC}"
echo -e "  http://localhost:${API_PORT:-3000}/api/status"

echo -e "${YELLOW}To stop the environment:${NC}"
echo -e "  ./stop-helius.sh"

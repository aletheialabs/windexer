#!/bin/bash
set -e

# Default values (can be overridden by environment variables)
export NODES=${NODES:-3}
export INDEXERS=${INDEXERS:-2}
export BASE_PORT=${BASE_PORT:-9000}
export RPC_PORT=${RPC_PORT:-8899}
export WS_PORT=${WS_PORT:-8900}
export FAUCET_PORT=${FAUCET_PORT:-9910}
export INDEXER_BASE_PORT=${INDEXER_BASE_PORT:-10000}
export API_PORT=${API_PORT:-3000}
export RUST_LOG=${RUST_LOG:-info}

# Calculate BASE_PORT_PLUS_ONE
export BASE_PORT_PLUS_ONE=$((BASE_PORT + 1))

# Script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Starting wIndexer Docker Environment ===${NC}"
echo -e "${YELLOW}Configuration:${NC}"
echo -e "  Nodes: ${NODES}"
echo -e "  Indexers: ${INDEXERS}"
echo -e "  Base port: ${BASE_PORT}"
echo -e "  Validator RPC port: ${RPC_PORT}"
echo -e "  Validator WebSocket port: ${WS_PORT}"
echo -e "  Validator Faucet port: ${FAUCET_PORT}"
echo -e "  Indexer base port: ${INDEXER_BASE_PORT}"
echo -e "  API port: ${API_PORT}"
echo -e "  Log level: ${RUST_LOG}"

# Generate docker-compose.override.yml
echo -e "${YELLOW}Generating docker-compose configuration...${NC}"
./generate-compose.sh

# Build and start the containers
echo -e "${YELLOW}Building and starting services...${NC}"
docker-compose -f docker-compose.yml -f docker-compose.override.yml build
docker-compose -f docker-compose.yml -f docker-compose.override.yml up -d

echo -e "${GREEN}Services started!${NC}"
echo -e "${YELLOW}Services:${NC}"
echo -e "  Solana Validator: http://localhost:${RPC_PORT}"
echo -e "  Solana Faucet: http://localhost:${FAUCET_PORT}"

# Display node info
for i in $(seq 0 $((NODES-1))); do
  NODE_PORT=$((BASE_PORT + i*100))
  echo -e "  Node $i: http://localhost:$NODE_PORT/api/status"
done

# Display indexer info
for i in $(seq 1 $INDEXERS); do
  INDEXER_PORT=$((INDEXER_BASE_PORT + i))
  echo -e "  Indexer $i: http://localhost:${INDEXER_PORT}/api/status"
done

echo -e "${YELLOW}Running data generator...${NC}"
echo -e "  Check logs with: docker-compose logs -f data-generator"

echo -e "${YELLOW}To stop the environment:${NC}"
echo -e "  ./stop-docker.sh" 
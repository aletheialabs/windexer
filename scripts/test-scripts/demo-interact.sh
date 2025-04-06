#!/bin/bash
set -e

# Colors for better output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== wIndexer Network Demo ===${NC}"

# Check if network is running
check_network() {
  local endpoint=$1
  local name=$2
  echo -e "${YELLOW}Checking $name at $endpoint...${NC}"
  if curl -s "$endpoint" >/dev/null; then
    echo -e "${GREEN}✓ $name is running${NC}"
    return 0
  else
    echo -e "${RED}✗ $name is not responding${NC}"
    return 1
  fi
}

# Wait for network to be ready
echo -e "${YELLOW}Waiting for network to initialize...${NC}"
sleep 2

# Check validator
check_network "http://localhost:8899/health" "Solana Validator" || {
  echo -e "${RED}Error: Validator is not running. Start it with 'make run-validator-with-geyser'${NC}"
  exit 1
}

# Generate some test transactions
echo -e "\n${YELLOW}Generating test transactions...${NC}"
./scripts/test-scripts/generate-data.sh

# Check node status
echo -e "\n${YELLOW}Network Stats:${NC}"

# Try to get node status
check_network "http://localhost:9000/api/status" "Node 0"
check_network "http://localhost:9001/api/status" "Node 1"

# Check indexer status
echo -e "\n${YELLOW}Indexer Stats:${NC}"
check_network "http://localhost:10001/api/status" "Indexer 1"
check_network "http://localhost:10101/api/status" "Indexer 2"

# Show recent transactions (if any)
echo -e "\n${YELLOW}Recent Transactions:${NC}"
curl -s http://localhost:10001/api/transactions || echo -e "${RED}  No transactions available${NC}"

echo -e "\n${GREEN}Demo complete! The network is running and ready for interaction.${NC}"
echo -e "You can use these endpoints for further interaction:"
echo -e "  - Node 0 API: http://localhost:9000"
echo -e "  - Node 1 API: http://localhost:9001"
echo -e "  - Indexer 1 API: http://localhost:10001"
echo -e "  - Indexer 2 API: http://localhost:10101"
echo -e "  - Validator RPC: http://localhost:8899"

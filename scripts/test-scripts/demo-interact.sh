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
echo -e "\n${YELLOW}Network Nodes:${NC}"

# Try to get node status for the p2p nodes on base port 9000
for i in 0 1 2 3; do
  port=$((9000 + i))
  check_network "http://localhost:$port/api/status" "Node $i (port $port)" || break
done

# Show recent transactions from any available node
echo -e "\n${YELLOW}Recent Transactions:${NC}"
for i in 0 1 2 3; do
  port=$((9000 + i))
  transactions=$(curl -s http://localhost:$port/api/transactions 2>/dev/null)
  if [ -n "$transactions" ] && [ "$transactions" != "null" ]; then
    echo -e "${GREEN}Found transactions from Node $i:${NC}"
    echo $transactions | jq '.' 2>/dev/null || echo $transactions
    break
  fi
done

echo -e "\n${GREEN}Demo complete! The network is running and ready for interaction.${NC}"
echo -e "You can use these base endpoints for further interaction:"
echo -e "  - P2P Nodes: http://localhost:9000, http://localhost:10001, etc."
echo -e "  - Validator RPC: http://localhost:8899"

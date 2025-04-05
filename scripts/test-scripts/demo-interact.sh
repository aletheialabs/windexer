#!/bin/bash
set -e

# Colors for better output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== wIndexer Network Demo ===${NC}"

# Wait for network to be ready
sleep 2

# Generate some test transactions
echo -e "${YELLOW}Generating test transactions...${NC}"
./scripts/test-scripts/generate-data.sh

# Check node status
echo -e "\n${YELLOW}Network Stats:${NC}"

# Try to get node status
echo -e "${BLUE}Node 0 stats:${NC}"
curl -s http://localhost:9000/api/status || echo "  Node not responding"

echo -e "\n${BLUE}Node 1 stats:${NC}"
curl -s http://localhost:9001/api/status || echo "  Node not responding"

# Check indexer status
echo -e "\n${YELLOW}Indexer Stats:${NC}"

echo -e "${BLUE}Indexer 1 stats:${NC}"
curl -s http://localhost:10001/api/status || echo "  Indexer not responding"

echo -e "\n${BLUE}Indexer 2 stats:${NC}"
curl -s http://localhost:10101/api/status || echo "  Indexer not responding"

# Show recent transactions (if any)
echo -e "\n${YELLOW}Recent Transactions:${NC}"
curl -s http://localhost:10001/api/transactions || echo "  No transactions available"

echo -e "\n${GREEN}Demo complete! The network is running and ready for interaction.${NC}"
echo -e "You can use these endpoints for further interaction:"
echo -e "  - Node 0 API: http://localhost:9000"
echo -e "  - Node 1 API: http://localhost:9001"
echo -e "  - Indexer 1 API: http://localhost:10001"
echo -e "  - Indexer 2 API: http://localhost:10101"
echo -e "  - Validator RPC: http://localhost:8899"

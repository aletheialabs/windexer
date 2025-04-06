#!/bin/bash
# Script to run TypeScript examples for windexer

# Set text colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Navigate to the TypeScript examples directory
SCRIPT_DIR="$(dirname "$0")"
TS_DIR="$SCRIPT_DIR/../examples/typescript"

echo -e "${BLUE}====================================${NC}"
echo -e "${BLUE}    wIndexer TypeScript Examples    ${NC}"
echo -e "${BLUE}====================================${NC}"

# Check if directory exists
if [ ! -d "$TS_DIR" ]; then
  echo -e "${RED}❌ TypeScript examples directory not found: $TS_DIR${NC}"
  exit 1
fi

# Navigate to the TypeScript directory
cd "$TS_DIR" || exit 1

# Check if the validator is running
echo -e "${YELLOW}Checking if Solana validator is running...${NC}"
solana validators --url http://localhost:8899 &>/dev/null
if [ $? -ne 0 ]; then
  echo -e "${RED}❌ Solana validator does not seem to be running on port 8899${NC}"
  echo -e "${YELLOW}Please start the validator:${NC}"
  echo -e "    make run-validator-with-geyser"
  exit 1
else
  echo -e "${GREEN}✓ Solana validator is running${NC}"
fi

# Run TypeScript examples
echo -e "\n${YELLOW}1. Running query-solana.ts${NC}"
npx ts-node query-solana.ts

echo -e "\n${YELLOW}2. Generating transactions with simple-tx.ts${NC}"
npx ts-node simple-tx.ts

echo -e "\n${YELLOW}3. Querying wIndexer API${NC}"
npx ts-node query-windexer.ts

echo -e "\n${YELLOW}4. Running websocket-subscribe.ts (will run for 10 seconds)${NC}"
# Run websocket with timeout to not block forever
timeout 10s npx ts-node websocket-subscribe.ts || true

echo -e "\n${GREEN}All TypeScript examples completed successfully!${NC}"
echo -e "For more information, check the README.md file"

# scripts/test-scripts/start-network.sh
#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Define directories
DATA_DIR="./data"
LOG_DIR="$DATA_DIR/logs"

echo -e "${BLUE}=== Starting wIndexer Network ===${NC}"

# Create directories
mkdir -p "$DATA_DIR"
mkdir -p "$LOG_DIR"

# Cleanup existing processes
echo -e "${YELLOW}Cleaning up old processes...${NC}"
pkill -f "solana-test-validator" 2>/dev/null || true
pkill -f "target/debug/node" 2>/dev/null || true
sleep 2

# Clear old data
rm -rf $DATA_DIR/*
mkdir -p "$DATA_DIR"
mkdir -p "$LOG_DIR"

# Start Solana validator
echo -e "${YELLOW}Starting Solana validator with Geyser plugin...${NC}"
make run-validator-with-geyser > "$LOG_DIR/validator.log" 2>&1 &
VALIDATOR_PID=$!
echo -e "${GREEN}Validator started with PID $VALIDATOR_PID${NC}"

# Wait for validator to start
echo -e "${YELLOW}Waiting for validator to start...${NC}"
for i in {1..30}; do
  if solana --url http://localhost:8899 cluster-version &>/dev/null; then
    echo -e "${GREEN}Validator is ready!${NC}"
    break
  fi
  if [ $i -eq 30 ]; then
    echo -e "${RED}Validator failed to start in time. Check logs at $LOG_DIR/validator.log${NC}"
    exit 1
  fi
  echo -n "."
  sleep 1
done

# Start nodes using Makefile targets
echo -e "${YELLOW}Starting nodes using Makefile...${NC}"
make run-local-network &
NETWORK_PID=$!
echo -e "${GREEN}Network started${NC}"

echo -e "${GREEN}Network fully initialized!${NC}"
echo -e "Access services at:"
echo -e "  - Node 0: http://localhost:9000"
echo -e "  - Node 1: http://localhost:10001"
echo -e "  - Validator RPC: http://localhost:8899"
echo -e ""
echo -e "Run './scripts/test-scripts/generate-data.sh' to create test transactions"
echo -e "Run './scripts/test-scripts/demo-interact.sh' to check network status"
echo -e "View logs with 'tail -f $LOG_DIR/validator.log'"
echo -e ""
echo -e "${YELLOW}Press Ctrl+C to stop the network (then run ./scripts/test-scripts/stop-network.sh)${NC}"

# Wait for Ctrl+C
wait $VALIDATOR_PID

# scripts/test-scripts/start-network.sh
#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Store PIDs for cleanup
declare -a PIDS=()

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
pkill -f "windexer" 2>/dev/null || true
sleep 2

# Clear old data
rm -rf $DATA_DIR/*
mkdir -p "$DATA_DIR"
mkdir -p "$LOG_DIR"

# Setup Geyser config
echo -e "${YELLOW}Setting up Geyser plugin configuration...${NC}"
./scripts/test-scripts/setup-geyser.sh

# Build components
echo -e "${YELLOW}Building components...${NC}"
cargo build --package windexer-geyser || { 
  echo -e "${RED}Failed to build Geyser plugin${NC}"
  exit 1
}

# Start Solana validator with Geyser plugin
echo -e "${YELLOW}Starting Solana validator with Geyser plugin...${NC}"
solana-test-validator \
  --reset \
  --rpc-port 8899 \
  --faucet-port 8990 \
  --geyser-plugin-config config/geyser/windexer-geyser-config.json \
  > "$LOG_DIR/validator.log" 2>&1 &

validator_pid=$!
PIDS+=($validator_pid)
echo -e "${GREEN}Validator started with PID $validator_pid${NC}"

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

# Start P2P network nodes
echo -e "${YELLOW}Starting P2P network nodes...${NC}"
for i in 0 1; do
  cargo run --bin node -- \
    --index $i \
    --base-port 9000 \
    --enable-tip-route \
    --data-dir "$DATA_DIR/node_$i" \
    > "$LOG_DIR/node_$i.log" 2>&1 &
  
  node_pid=$!
  PIDS+=($node_pid)
  echo -e "${GREEN}Node $i started with PID $node_pid${NC}"
done

# Wait for network to initialize
echo -e "${YELLOW}Waiting for P2P network to initialize...${NC}"
sleep 5

# Start indexers
echo -e "${YELLOW}Starting indexers...${NC}"
for i in 1 2; do
  cargo run --bin indexer -- \
    --index $i \
    --base-port $((10000 + (i-1)*100)) \
    --bootstrap-peers 127.0.0.1:9000 \
    --data-dir "$DATA_DIR/indexer_$i" \
    > "$LOG_DIR/indexer_$i.log" 2>&1 &
  
  indexer_pid=$!
  PIDS+=($indexer_pid)
  echo -e "${GREEN}Indexer $i started with PID $indexer_pid${NC}"
done

# Function to clean up processes on exit
cleanup() {
  echo -e "${YELLOW}Cleaning up processes...${NC}"
  for pid in "${PIDS[@]}"; do
    if ps -p $pid > /dev/null; then
      echo -e "${YELLOW}Stopping PID $pid${NC}"
      kill $pid 2>/dev/null || true
    fi
  done
  echo -e "${GREEN}Cleanup complete${NC}"
}

# Set up trap for SIGINT and SIGTERM
trap cleanup SIGINT SIGTERM

echo -e "${GREEN}Network fully initialized!${NC}"
echo -e "Access services at:"
echo -e "  - Indexer 1: http://localhost:10001"
echo -e "  - Indexer 2: http://localhost:10101"
echo -e "  - Node 0: http://localhost:9000"
echo -e "  - Node 1: http://localhost:9001"
echo -e "  - Validator RPC: http://localhost:8899"
echo -e ""
echo -e "Run './scripts/test-scripts/generate-data.sh' to create test transactions"
echo -e "Run './scripts/test-scripts/demo-interact.sh' to check network status"
echo -e "View logs with 'tail -f $LOG_DIR/<component>.log'"
echo -e ""
echo -e "${YELLOW}Press Ctrl+C to stop the network${NC}"

# Wait for Ctrl+C
wait

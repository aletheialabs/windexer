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

# Create directories - make sure this happens BEFORE any log files are written
mkdir -p "$DATA_DIR"
mkdir -p "$LOG_DIR"  # Explicitly create the logs directory

# Cleanup existing processes
echo "Cleaning up old processes..."
pkill -f "solana-test-validator" 2>/dev/null || true
pkill -f "windexer" 2>/dev/null || true
sleep 2

# Clear old data but preserve the directory structure
rm -rf $DATA_DIR/*
mkdir -p "$DATA_DIR"
mkdir -p "$LOG_DIR"  # Create logs directory again after clearing data

# Build Geyser plugin
echo "Building Geyser plugin..."
cargo build --package windexer-geyser

# Start Solana validator with Geyser plugin
echo "Starting Solana validator with wIndexer Geyser plugin..."
solana-test-validator \
  --reset \
  --rpc-port 8899 \
  --faucet-port 8990 \
  --geyser-plugin-config config/geyser/windexer-geyser-config.json \
  > "$LOG_DIR/validator.log" 2>&1 &

validator_pid=$!
echo "Validator started with PID $validator_pid"

# Wait for validator to start
echo "Waiting for validator to start..."
for i in {1..30}; do
  if solana --url http://localhost:8899 cluster-version 2>/dev/null; then
    echo "Validator is ready!"
    break
  fi
  if [ $i -eq 30 ]; then
    echo "Validator failed to start in time. Check logs at $LOG_DIR/validator.log"
    exit 1
  fi
  echo -n "."
  sleep 1
done

# Start P2P network nodes
echo "Starting P2P network nodes..."
for i in 0 1; do
  cargo run --bin node -- \
    --index $i \
    --base-port 9000 \
    --enable-tip-route \
    --data-dir "$DATA_DIR/node_$i" \
    > "$LOG_DIR/node_$i.log" 2>&1 &
  
  echo "Node $i started"
done

# Wait for network to initialize
echo "Waiting for P2P network to initialize..."
sleep 5

# Start indexers
echo "Starting indexers..."
for i in 1 2; do
  cargo run --bin indexer -- \
    --index $i \
    --base-port $((10000 + (i-1)*100)) \
    --bootstrap-peers 127.0.0.1:9000 \
    --data-dir "$DATA_DIR/indexer_$i" \
    > "$LOG_DIR/indexer_$i.log" 2>&1 &
  
  echo "Indexer $i started"
done

echo "Network fully initialized!"
echo "Access indexers at:"
echo "  - Indexer 1: http://localhost:10001"
echo "  - Indexer 2: http://localhost:10101"
echo "  - Validator RPC: http://localhost:8899"
echo ""
echo "Run './scripts/test-scripts/generate-data.sh' to create test transactions"
echo "Use 'tail -f $LOG_DIR/<component>.log' to view logs"

# Function to check if a command exists
check_command() {
    if ! command -v $1 &> /dev/null; then
        echo -e "${YELLOW}Warning: $1 is not installed. Some functionality may be limited.${NC}"
        return 1
    fi
    return 0
}

# Function to wait for a service to be ready
wait_for_service() {
    local name=$1
    local port=$2
    local max_attempts=$3
    local attempt=1
    
    echo -e "${YELLOW}Waiting for $name to be ready...${NC}"
    
    # If netcat is available, use it
    if command -v nc &> /dev/null; then
        while ! nc -z localhost $port &>/dev/null; do
            if [ $attempt -gt $max_attempts ]; then
                echo -e "${RED}$name not ready after $max_attempts attempts. Continuing anyway...${NC}"
                return 1
            fi
            sleep 1
            ((attempt++))
        done
    # Otherwise, try to connect using bash's built-in /dev/tcp facility
    elif [ -n "$BASH_VERSION" ]; then
        while ! timeout 1 bash -c "echo > /dev/tcp/localhost/$port" &>/dev/null; do
            if [ $attempt -gt $max_attempts ]; then
                echo -e "${RED}$name not ready after $max_attempts attempts. Continuing anyway...${NC}"
                return 1
            fi
            sleep 1
            ((attempt++))
        done
    # If neither method is available, just wait a fixed time
    else
        echo -e "${YELLOW}Cannot check if $name is ready. Waiting ${max_attempts}s...${NC}"
        sleep $max_attempts
    fi
    
    echo -e "${GREEN}$name is ready!${NC}"
    return 0
}

# Function to clean up all processes
cleanup() {
    echo -e "${YELLOW}Cleaning up...${NC}"
    for pid in "${PIDS[@]}"; do
        if ps -p $pid > /dev/null; then
            echo -e "Stopping process $pid"
            kill $pid 2>/dev/null || true
        fi
    done
    echo -e "${GREEN}All processes stopped${NC}"
}

# Check prerequisites
check_command cargo || true
check_command solana-test-validator || echo -e "${RED}Error: solana-test-validator is required${NC}"

# Make sure we're in the project root
cd "$(dirname "$0")/.."

# Create the setup-geyser script executable if needed
chmod +x scripts/test-scripts/setup-geyser.sh

# Setup Geyser config
echo -e "${BLUE}Setting up Geyser plugin configuration...${NC}"
./scripts/test-scripts/setup-geyser.sh

# Build everything
echo -e "${BLUE}Building all components...${NC}"
cargo build || { echo -e "${RED}Build failed${NC}"; exit 1; }

# Setup directories
mkdir -p data

# Start the first node in the background
echo -e "${BLUE}Starting node 1...${NC}"
make run-node-1 &
NODE1_PID=$!
PIDS+=($NODE1_PID)
sleep 2

# Check if node 1 is running
if ! ps -p $NODE1_PID > /dev/null; then
    echo -e "${RED}Node 1 failed to start. Check the logs.${NC}"
    cleanup
    exit 1
fi

# Start the second node in the background
echo -e "${BLUE}Starting node 2...${NC}"
make run-node-2 &
NODE2_PID=$!
PIDS+=($NODE2_PID)
sleep 2

# Check if node 2 is running
if ! ps -p $NODE2_PID > /dev/null; then
    echo -e "${RED}Node 2 failed to start. Check the logs.${NC}"
    cleanup
    exit 1
fi

# Start the validator with Geyser plugin in the background
echo -e "${BLUE}Starting Solana validator with Geyser plugin...${NC}"
make run-validator-with-geyser &
VALIDATOR_PID=$!
PIDS+=($VALIDATOR_PID)

# Wait for validator to be ready (RPC port 8899)
wait_for_service "Solana validator" 8899 30 || true  # Continue even if waiting fails

# Start the first indexer in the background
echo -e "${BLUE}Starting indexer 1...${NC}"
make run-indexer-1 &
INDEXER1_PID=$!
PIDS+=($INDEXER1_PID)
sleep 2

# Check if indexer 1 is running
if ! ps -p $INDEXER1_PID > /dev/null; then
    echo -e "${RED}Indexer 1 failed to start. Check the logs.${NC}"
    cleanup
    exit 1
fi

# Start the second indexer in the background
echo -e "${BLUE}Starting indexer 2...${NC}"
make run-indexer-2 &
INDEXER2_PID=$!
PIDS+=($INDEXER2_PID)
sleep 2

# Check if indexer 2 is running
if ! ps -p $INDEXER2_PID > /dev/null; then
    echo -e "${RED}Indexer 2 failed to start. Check the logs.${NC}"
    cleanup
    exit 1
fi

echo -e "${GREEN}✅ Full demo is running!${NC}"
echo -e "${YELLOW}Available services:${NC}"
echo -e "  - Solana validator: http://localhost:8899"
echo -e "  - Node 1 p2p: 127.0.0.1:9001"
echo -e "  - Node 2 p2p: 127.0.0.1:9002"
echo -e "  - Indexer 1 API: http://localhost:10001"
echo -e "  - Indexer 2 API: http://localhost:10002"
echo -e ""
echo -e "Demo transactions will be indexed automatically as they occur."
echo -e "Press ${YELLOW}Ctrl+C${NC} to stop all processes"

# Set up trap to catch Ctrl+C and clean up
trap cleanup INT TERM

# Keep the script running
wait $NODE1_PID $NODE2_PID $VALIDATOR_PID $INDEXER1_PID $INDEXER2_PID 
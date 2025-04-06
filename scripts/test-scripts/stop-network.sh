#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Stopping wIndexer Network ===${NC}"

# Function to kill processes by pattern
kill_processes() {
  local pattern=$1
  local name=$2
  echo -e "${YELLOW}Stopping $name processes...${NC}"
  
  pids=$(pgrep -f "$pattern" 2>/dev/null || echo "")
  if [ -n "$pids" ]; then
    echo -e "${GREEN}Found processes: $pids${NC}"
    pkill -f "$pattern" 2>/dev/null && echo -e "${GREEN}Successfully stopped $name processes${NC}" || echo -e "${RED}Failed to stop some $name processes${NC}"
  else
    echo -e "${YELLOW}No $name processes found${NC}"
  fi
}

# Stop the validator
kill_processes "solana-test-validator" "Solana validator"

# Stop any running nodes 
kill_processes "target/debug/node" "Node"

# Stop any make processes that might be running the network
# This helps clean up processes started via Makefile targets
kill_processes "make run-" "Make"

echo -e "${GREEN}All network processes stopped${NC}"
echo -e "${YELLOW}You may need to manually check for remaining processes with:${NC}"
echo -e "  ps aux | grep -E 'solana|node'" 
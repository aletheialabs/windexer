#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Stopping wIndexer Network ===${NC}"

# Cleanup existing processes
echo -e "${YELLOW}Stopping validator...${NC}"
pkill -f "solana-test-validator" 2>/dev/null || echo -e "${YELLOW}No validator process found${NC}"

echo -e "${YELLOW}Stopping windexer processes...${NC}"
pkill -f "windexer" 2>/dev/null || echo -e "${YELLOW}No windexer processes found${NC}"

# Additional search for node and indexer specific processes
echo -e "${YELLOW}Stopping any remaining processes...${NC}"
pkill -f "node --index" 2>/dev/null || true
pkill -f "indexer --index" 2>/dev/null || true

echo -e "${GREEN}All network processes stopped${NC}" 
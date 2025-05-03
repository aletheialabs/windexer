#!/bin/bash
set -e
# Script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Stopping wIndexer Helius Environment ===${NC}"

# Option to clean data
if [ "$1" == "--clean" ]; then
  echo -e "${YELLOW}Stopping and removing containers, networks, volumes, and data...${NC}"
  docker-compose -f docker-compose.helius.yml -f docker-compose.helius.override.yml down -v

  echo -e "${YELLOW}Cleaning data directories...${NC}"
  read -p "Are you sure you want to delete all data directories? (y/n) " -n 1 -r
  echo
  if [[ $REPLY =~ ^[Yy]$ ]]; then
    rm -rf ${STORAGE_DIR:-../../data}/node_* ${STORAGE_DIR:-../../data}/indexer_* ${STORAGE_DIR:-../../data}/postgres
    echo -e "${GREEN}Data directories cleaned!${NC}"
  else
    echo -e "${YELLOW}Data directories preserved.${NC}"
  fi

  echo -e "${GREEN}Environment stopped and cleaned!${NC}"
else
  echo -e "${YELLOW}Stopping containers...${NC}"
  docker-compose -f docker-compose.helius.yml -f docker-compose.helius.override.yml down

  echo -e "${GREEN}Environment stopped!${NC}"
  echo -e "${YELLOW}To remove all data, use:${NC}"
  echo -e "  ./stop-helius.sh --clean"
fi

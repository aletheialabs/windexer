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

echo -e "${BLUE}=== Stopping wIndexer Docker Environment ===${NC}"

# Option to clean data
if [ "$1" == "--clean" ]; then
  echo -e "${YELLOW}Stopping and removing containers, networks, volumes, and data...${NC}"
  docker-compose -f docker-compose.yml -f docker-compose.override.yml down -v
  
  echo -e "${YELLOW}Cleaning data directories...${NC}"
  rm -rf ../../data/node_* ../../data/indexer_* ../../data/validator ../../data/geyser
  
  echo -e "${GREEN}Environment stopped and data cleaned!${NC}"
else
  echo -e "${YELLOW}Stopping containers...${NC}"
  docker-compose -f docker-compose.yml -f docker-compose.override.yml down
  
  echo -e "${GREEN}Environment stopped!${NC}"
  echo -e "${YELLOW}To remove all data, use:${NC}"
  echo -e "  ./stop-docker.sh --clean"
fi 
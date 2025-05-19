#!/bin/bash
set -e

# Script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Backup timestamp
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
BACKUP_DIR="./backups/deployment_$TIMESTAMP"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Backing Up wIndexer Deployment ===${NC}"
echo -e "${YELLOW}Creating backup in ${BACKUP_DIR}${NC}"

# Create backup directory structure
mkdir -p ${BACKUP_DIR}/{config,data,volumes}

# Backup configuration files
echo -e "${YELLOW}Backing up configuration files...${NC}"
cp -r ./config ${BACKUP_DIR}/
cp -f docker-compose.yml ${BACKUP_DIR}/
cp -f docker-compose.override.yml ${BACKUP_DIR}/ 2>/dev/null || true
cp -f .env ${BACKUP_DIR}/ 2>/dev/null || true
echo "Configuration files backed up."

# Get container names
CONTAINERS=$(docker-compose ps --services)

# Backup Docker volumes
echo -e "${YELLOW}Backing up Docker volumes...${NC}"

# Create a temporary container for backing up volumes
docker run --rm -v $(pwd)/${BACKUP_DIR}/volumes:/backup -v postgres-data:/postgres-data -v geyser-plugin:/geyser-plugin alpine:latest sh -c "
  echo 'Starting volume backup...'
  cd / && 
  tar czf /backup/postgres-data.tar.gz postgres-data 2>/dev/null || echo 'No postgres-data volume to backup.'
  tar czf /backup/geyser-plugin.tar.gz geyser-plugin 2>/dev/null || echo 'No geyser-plugin volume to backup.'
  echo 'Volume backup completed.'
"

echo "Docker volumes backed up."

# Backup relevant data directories (selectively, to avoid backing up huge blockchain data)
echo -e "${YELLOW}Backing up selective data directories...${NC}"

# Create a list of directories to backup
mkdir -p ${BACKUP_DIR}/data
for dir in node_0 node_1 node_2 indexer_1 indexer_2; do
  if [ -d "../../data/$dir" ]; then
    echo "Backing up data/$dir configuration files only..."
    mkdir -p ${BACKUP_DIR}/data/$dir
    find ../../data/$dir -type f -name "*.json" -o -name "*.toml" -o -name "*.yml" -o -name "*.yaml" | xargs -I {} cp --parents {} ${BACKUP_DIR}/
  fi
done

echo "Data directories backed up (configuration files only)."

# Save container information
echo -e "${YELLOW}Saving container information...${NC}"
mkdir -p ${BACKUP_DIR}/container_info
for service in $CONTAINERS; do
  container_id=$(docker-compose ps -q $service)
  if [ ! -z "$container_id" ]; then
    echo "Saving information for $service..."
    docker inspect $container_id > ${BACKUP_DIR}/container_info/${service}.json
  fi
done

echo "Container information saved."

# Create a compressed archive of the backup
echo -e "${YELLOW}Creating compressed archive...${NC}"
cd $(dirname ${BACKUP_DIR})
tar czf deployment_${TIMESTAMP}.tar.gz $(basename ${BACKUP_DIR})
cd - > /dev/null

echo -e "${GREEN}Backup completed successfully!${NC}"
echo -e "Backup location: ${BACKUP_DIR}"
echo -e "Compressed archive: $(dirname ${BACKUP_DIR})/deployment_${TIMESTAMP}.tar.gz"

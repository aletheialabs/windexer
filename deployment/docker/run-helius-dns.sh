#!/bin/bash

# Run the Helius stack with DNS configuration
# This script starts the API, node, and indexer services with DNS using the Helius API

# Ensure we're in the right directory
cd "$(dirname "$0")"

# Set environment variables
export HELIUS_API_KEY="${HELIUS_API_KEY:-b7cd157e-629d-4d30-bc54-58c3361380ed}"
export API_PORT=3000
export DNS_NAME="test-may-us-01.windnetwork.ai"

echo "Starting the Helius API + node + indexer stack with DNS: $DNS_NAME"
echo "Helius API Key: ${HELIUS_API_KEY:0:5}...${HELIUS_API_KEY:(-5)}"

# Start the stack with DNS configuration
docker-compose -f docker-compose.helius.yml up -d

# Create a traefik container to handle routing if it doesn't exist
if ! docker ps | grep -q traefik; then
  echo "Adding Traefik for DNS routing..."
  docker-compose -f docker-compose.dns.override.yml up -d traefik
fi

echo ""
echo "Helius Stack started. You can access the following services:"
echo " - API Server:           http://$DNS_NAME:3000/api"
echo " - Node-0:               http://$DNS_NAME:9000"
echo " - Indexer-1:            http://$DNS_NAME:10001"
echo ""
echo "For more advanced routing, use the run-dns-stack.sh script instead." 
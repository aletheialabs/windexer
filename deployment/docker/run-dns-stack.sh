#!/bin/bash

# Run the solana stack with DNS configuration
# This script starts the validator, geyser, node, indexer, and API services with DNS

# Ensure we're in the right directory
cd "$(dirname "$0")"

# Set environment variables
export HELIUS_API_KEY="${HELIUS_API_KEY:-b7cd157e-629d-4d30-bc54-58c3361380ed}"
export RPC_PORT=8899
export WS_PORT=8900
export API_PORT=3000
export DNS_NAME="test-may-us-01.windnetwork.ai"

echo "Starting the Solana validator + node + indexer + API stack with DNS: $DNS_NAME"
echo "Helius API Key: ${HELIUS_API_KEY:0:5}...${HELIUS_API_KEY:(-5)}"

# Check if we need to build the validator image
if ! docker image inspect validator-image >/dev/null 2>&1; then
  echo "Building validator image..."
  docker build -t validator-image -f custom/Dockerfile.validator .
fi

# Create the configuration directory for local development
mkdir -p ./traefik

# Start the stack with DNS configuration
docker-compose -f docker-compose.yml -f docker-compose.dns.override.yml up -d

echo ""
echo "Stack started. You can access the following services:"
echo " - Solana Validator RPC: http://$DNS_NAME/rpc"
echo " - Solana Validator WS:  ws://$DNS_NAME/ws"
echo " - API Server:           http://$DNS_NAME/api"
echo " - Node-0:               http://$DNS_NAME/node"
echo " - Indexer:              http://$DNS_NAME/indexer"
echo ""
echo "For local testing, you may need to add entries to your /etc/hosts file:"
echo "127.0.0.1    $DNS_NAME"
echo ""
echo "Traefik Dashboard is available at: http://localhost:8080" 
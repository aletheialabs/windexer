#!/bin/bash

# Start script for Cambrian AVS Demo using actual Windexer code

echo "🚀 Starting Cambrian AVS Demo with real Windexer-Cambrian integration..."

# Create logs directory
mkdir -p logs

# Copy Docker compose template if it doesn't exist
if [ ! -f "docker-compose.yml" ]; then
    echo "Creating docker-compose.yml..."
    cp ../../examples/cambrian-avs-demo/docker-compose.yml ./
fi

# Make sure Dockerfiles exist
if [ ! -f "Dockerfile.avs" ]; then
    cp ../../examples/cambrian-avs-demo/Dockerfile.avs ./
fi

if [ ! -f "Dockerfile.windexer" ]; then
    cp ../../examples/cambrian-avs-demo/Dockerfile.windexer ./
fi

# Start docker containers with build
echo "Building and starting Docker containers..."
docker-compose build || echo "Warning: Docker build had issues, trying to start anyway"
docker-compose up -d || (sleep 5 && docker-compose up -d)

echo "⌛ Waiting for services to start..."
sleep 10

# Log to file with proper path
touch logs/avs.log
docker-compose logs -f > logs/avs.log 2>&1 &

echo """
✅ Cambrian AVS Demo is running with real Windexer code!

To interact with the demo:
- Submit a proposal: ./scripts/submit-proposal.sh [payload-name]
- View logs: docker-compose logs -f
- Stop the demo: ./scripts/stop.sh

Available payloads:
- basic
- nft
- mpl
- spl
- update-nft
""" 
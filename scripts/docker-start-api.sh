#!/bin/bash

# Windexer API Docker Startup script
# This script uses docker-compose to start the Windexer API with a Helius API key

# Check if Helius API key is provided
if [ -z "$HELIUS_API_KEY" ]; then
    echo "Error: HELIUS_API_KEY environment variable is not set"
    echo "Usage: HELIUS_API_KEY=your-api-key ./scripts/docker-start-api.sh"
    exit 1
fi

# Set default port if not specified
export API_PORT=${API_PORT:-3000}

# Set log level if not specified
export RUST_LOG=${RUST_LOG:-info}

echo "Starting Windexer API in Docker on port $API_PORT with Helius API key"
echo "Log level: $RUST_LOG"

# Change to the docker directory
cd deployment/docker

# Run docker-compose with the Helius configuration
docker-compose -f docker-compose.helius.yml up -d --build api-server

# Check if the API server started successfully
if [ $? -eq 0 ]; then
    echo "API server started successfully. Listening on port $API_PORT"
    echo "To view logs, run: docker logs -f api-server"
    echo "To stop the service, run: docker-compose -f deployment/docker/docker-compose.helius.yml down"
else
    echo "Failed to start API server"
    exit 1
fi 
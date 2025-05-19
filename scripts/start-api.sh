#!/bin/bash

# Windexer API Startup script
# This script starts the Windexer API locally with a Helius API key

# Check if Helius API key is provided
if [ -z "$HELIUS_API_KEY" ]; then
    echo "Error: HELIUS_API_KEY environment variable is not set"
    echo "Usage: HELIUS_API_KEY=your-api-key ./scripts/start-api.sh"
    exit 1
fi

# Set default port if not specified
API_PORT=${API_PORT:-3000}

# Set log level if not specified
RUST_LOG=${RUST_LOG:-info}

echo "Starting Windexer API on port $API_PORT with Helius API key"
echo "Log level: $RUST_LOG"

# Run the API with the provided environment variables
RUST_LOG=$RUST_LOG \
API_PORT=$API_PORT \
HELIUS_API_KEY=$HELIUS_API_KEY \
cargo run --release -p windexer-api

# Exit with the cargo run exit code
exit $? 
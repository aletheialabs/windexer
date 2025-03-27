#!/bin/bash

# Stop the Cambrian AVS Demo

echo "🛑 Stopping Cambrian AVS Demo..."
docker-compose down

echo "✅ Demo stopped. Run './scripts/start.sh' to start it again." 
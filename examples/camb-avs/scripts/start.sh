#!/bin/bash

# Start script for Cambrian AVS Demo

echo "🚀 Starting Cambrian AVS Demo..."

# Start docker containers
docker-compose up -d

echo "⌛ Waiting for services to start..."
sleep 5

echo """
✅ Cambrian AVS Demo is running!

To interact with the demo:
- Submit a proposal: ./scripts/submit-proposal.sh [payload-name]
- View logs: docker-compose logs -f
- Stop the demo: ./scripts/stop.sh

Available payloads:
- basic (payload-demo.ts)
- nft (payload-nft-demo.ts)
- mpl (payload-mpl-demo.ts)
- spl (payload-spl-demo.ts)
- update-nft (payload-update-mint-nft-auth.ts)
""" 
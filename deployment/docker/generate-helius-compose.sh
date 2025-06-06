#!/bin/bash
set -e
# Default values
NODES=${NODES:-3}
INDEXERS=${INDEXERS:-2}
BASE_PORT=${BASE_PORT:-9000}
INDEXER_BASE_PORT=${INDEXER_BASE_PORT:-10000}
# Output file
OUTPUT_FILE="docker-compose.helius.override.yml"
echo "Generating $OUTPUT_FILE for $NODES nodes and $INDEXERS indexers..."
# Start the YAML file
cat > $OUTPUT_FILE << EOL
# AUTO-GENERATED FILE - DO NOT EDIT DIRECTLY
# Generated with generate-helius-compose.sh
services:
  # Base node definition
  node-base: &node-base
    build:
      context: ../..
      dockerfile: deployment/docker/Dockerfile.node
    environment:
      - RUST_LOG=${RUST_LOG:-info}
      - SOLANA_RPC_URL=${SOLANA_RPC_URL}
      - SOLANA_WS_URL=${SOLANA_WS_URL}
      - NETWORK=${NETWORK:-mainnet}
      - BASE_PORT=${BASE_PORT:-9000}
    networks:
      - windexer-network
    restart: unless-stopped
EOL

# First node (node-0) definition
cat >> $OUTPUT_FILE << EOL
  # First node (node-0)
  node-0:
    <<: *node-base
    container_name: windexer-node-0
    volumes:
      - ${STORAGE_DIR:-/home/winuser/windexer/data}/node_0:/app/data/node_0
    ports:
      - "${BASE_PORT:-9000}:9000"
      - "${BASE_PORT_PLUS_ONE:-9001}:9001"
    command: node --index 0 --base-port 9000 --enable-tip-route
EOL

# Base indexer definition
cat >> $OUTPUT_FILE << EOL
  # Base indexer definition
  indexer-base: &indexer-base
    build:
      context: ../..
      dockerfile: deployment/docker/Dockerfile.node
    environment:
      - RUST_LOG=${RUST_LOG:-info}
      - SOLANA_RPC_URL=${SOLANA_RPC_URL}
      - SOLANA_WS_URL=${SOLANA_WS_URL}
      - NETWORK=${NETWORK:-mainnet}
      - BASE_PORT=${BASE_PORT:-9000}
    networks:
      - windexer-network
    depends_on:
      - node-0
    restart: unless-stopped
EOL

# Generate node services (start from node-1 since node-0 is already in the base file)
for i in $(seq 1 $((NODES-1))); do
  NODE_PORT=$((BASE_PORT + i*100))
  NODE_PORT_PLUS_ONE=$((NODE_PORT + 1))
  cat >> $OUTPUT_FILE << EOL
  # Node $i
  node-$i:
    <<: *node-base
    container_name: windexer-node-$i
    volumes:
      - ${STORAGE_DIR:-/home/winuser/windexer/data}/node_$i:/app/data/node_$i
    ports:
      - "$NODE_PORT:9000"
      - "$NODE_PORT_PLUS_ONE:9001"
    command: node --index $i --base-port 9000 --enable-tip-route --bootstrap-peers node-0:9000
EOL
done

# Generate indexer services
for i in $(seq 1 $INDEXERS); do
  INDEXER_PORT=$((INDEXER_BASE_PORT + i))
  cat >> $OUTPUT_FILE << EOL
  # Indexer $i
  indexer-$i:
    <<: *indexer-base
    container_name: windexer-indexer-$i
    volumes:
      - ${STORAGE_DIR:-/home/winuser/windexer/data}/indexer_$i:/app/data/indexer_$i
    ports:
      - "$INDEXER_PORT:$INDEXER_PORT"
    command: indexer --index $i --bootstrap-peers node-0:9000 --base-port $INDEXER_PORT
EOL
done

echo "Generated $OUTPUT_FILE with $NODES nodes and $INDEXERS indexers"
echo "Use with: docker-compose -f docker-compose.helius.yml -f $OUTPUT_FILE up -d"

#!/bin/bash
set -e

# Default values
NODES=${NODES:-3}
INDEXERS=${INDEXERS:-2}
BASE_PORT=${BASE_PORT:-9000}
INDEXER_BASE_PORT=${INDEXER_BASE_PORT:-10000}

# Output file
OUTPUT_FILE="docker-compose.override.yml"

echo "Generating docker-compose.override.yml for $NODES nodes and $INDEXERS indexers..."

# Start the YAML file
cat > $OUTPUT_FILE << EOL
# AUTO-GENERATED FILE - DO NOT EDIT DIRECTLY
# Generated with generate-compose.sh

services:
  # Base node definition
  node-base: &node-base
    build:
      context: ../..
      dockerfile: deployment/docker/Dockerfile.node
    environment:
      - RUST_LOG=${RUST_LOG:-info}
      - SOLANA_RPC_URL=http://solana-validator:8899
      - SOLANA_WS_URL=ws://solana-validator:8900
      - BASE_PORT=${BASE_PORT:-9000}
    networks:
      - windexer-network
    depends_on:
      solana-validator:
        condition: service_healthy
    deploy:
      replicas: 0

  # Base indexer definition
  indexer-base: &indexer-base
    build:
      context: ../..
      dockerfile: deployment/docker/Dockerfile.node
    environment:
      - RUST_LOG=${RUST_LOG:-info}
      - SOLANA_RPC_URL=http://solana-validator:8899
      - SOLANA_WS_URL=ws://solana-validator:8900
      - BASE_PORT=${BASE_PORT:-9000}
    networks:
      - windexer-network
    depends_on:
      - node-0
    deploy:
      replicas: 0
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
      - ../../data/node_$i:/app/data/node_$i
    ports:
      - "$NODE_PORT:9000"
      - "$NODE_PORT_PLUS_ONE:9001"
    command: node --index $i --base-port 9000 --enable-tip-route --bootstrap-peers node-0:9000
    deploy:
      replicas: 1
    depends_on:
      - node-0

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
      - ../../data/indexer_$i:/app/data/indexer_$i
    ports:
      - "$INDEXER_PORT:$INDEXER_PORT"
    command: indexer --index $i --bootstrap-peers node-0:9000 --base-port $INDEXER_PORT
    deploy:
      replicas: 1

EOL
done

echo "Generated $OUTPUT_FILE with $NODES nodes and $INDEXERS indexers"
echo "Use with: docker-compose -f docker-compose.yml -f $OUTPUT_FILE up -d" 
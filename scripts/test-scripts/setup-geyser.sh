#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Setting up wIndexer Geyser Plugin ===${NC}"

# Create Geyser plugin config directory
mkdir -p config/geyser

echo -e "${YELLOW}Creating Geyser plugin configuration...${NC}"

cat > config/geyser/windexer-geyser-config.json << EOL
{
  "libpath": "target/debug/libwindexer_geyser.so",
  "network": {
    "node_id": "geyser-plugin",
    "listen_addr": "127.0.0.1:9876",
    "rpc_addr": "127.0.0.1:9877",
    "bootstrap_peers": ["127.0.0.1:9000", "127.0.0.1:9001"],
    "data_dir": "./data/geyser",
    "solana_rpc_url": "http://localhost:8899"
  },
  "accounts_selector": {
    "accounts": null,
    "owners": null
  },
  "transaction_selector": {
    "mentions": [],
    "include_votes": false
  },
  "thread_count": 4,
  "batch_size": 100,
  "use_mmap": true,
  "panic_on_error": false
}
EOL

echo -e "${GREEN}✓ Geyser plugin configuration created at config/geyser/windexer-geyser-config.json${NC}" 
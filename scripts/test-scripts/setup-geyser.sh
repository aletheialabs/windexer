#!/bin/bash

# Create Geyser plugin config directory
mkdir -p config/geyser

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

echo "Geyser plugin configuration created at config/geyser/windexer-geyser-config.json" 
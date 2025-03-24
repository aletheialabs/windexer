#!/bin/bash
set -e

echo "🔧 Setting up windexer-geyser plugin..."

mkdir -p config/geyser
mkdir -p windexer_geyser_setup

if [ ! -f "config/geyser/plugin-keypair.json" ]; then
    echo "Generating new plugin keypair..."
    solana-keygen new --no-passphrase -o config/geyser/plugin-keypair.json
fi

cat > config/geyser/windexer-geyser-config.json << EOF
{
  "libpath": "../../target/debug/libwindexer_geyser.so",
  "keypair": "./plugin-keypair.json",
  "host": "127.0.0.1",
  "no_processors": true,
  "network": {
    "node_id": "windexer-node",
    "listen_addr": "127.0.0.1:8900",
    "rpc_addr": "127.0.0.1:8901",
    "bootstrap_peers": [],
    "data_dir": "./windexer_geyser_setup",
    "solana_rpc_url": "http://127.0.0.1:8899"
  },
  "accounts_selector": {
    "accounts": []
  },
  "transaction_selector": {
    "mentions": []
  },
  "thread_count": 1,
  "batch_size": 100,
  "panic_on_error": false,
  "use_mmap": false,
  "metrics": {
    "enabled": false
  }
}
EOF

echo "Building plugin..."
cargo build --package windexer-geyser

export WINDEXER_SKIP_NETWORK=1

echo "✅ Setup complete!"
echo "Running solana-test-validator with windexer-geyser plugin..."
WINDEXER_SKIP_NETWORK=1 RUST_BACKTRACE=1 RUST_LOG=solana_geyser_plugin_manager=debug,windexer_geyser=debug \
solana-test-validator \
  --geyser-plugin-config config/geyser/windexer-geyser-config.json \
  --reset &

VALIDATOR_PID=$!
echo "Waiting for validator to initialize (PID: $VALIDATOR_PID)..."

for i in {1..60}; do  # Increased to 60 seconds
  if curl -s http://localhost:8899 -X POST -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' | grep -q "ok"; then
    echo "Validator RPC is responsive!"
    break
  fi
  if [ $i -eq 60 ]; then
    echo "Validator initialization timed out after 60 seconds."
    echo "Check test-ledger/validator.log for details."
    kill -9 $VALIDATOR_PID 2>/dev/null || true
    exit 1
  fi
  echo -n "."
  sleep 1
done

wait $VALIDATOR_PID
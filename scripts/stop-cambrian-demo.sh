#!/bin/bash
set -e

echo "===== Stopping Cambrian AVS Demo ====="

# Check if demo is running
if ! docker ps | grep -q "cambrian-avs"; then
  echo "ℹ️ Cambrian AVS demo is not running."
  exit 0
fi

echo "🛑 Stopping Cambrian AVS demo..."
cd examples/cambrian-avs-demo
./scripts/stop.sh
cd ../..

echo "✅ Cambrian AVS demo has been stopped" 
#!/bin/bash
set -e

echo "===== Cambrian AVS with Windexer Demo ====="
echo "🚀 This demo showcases the integration of Cambrian AVS with Windexer"

# Check required dependencies
echo "Checking dependencies..."
if ! command -v docker &> /dev/null; then
  echo "❌ Docker is not installed or not in PATH. This demo requires Docker."
  exit 1
fi

if ! command -v solana &> /dev/null; then
  echo "❌ Solana CLI is not installed or not in PATH. This demo requires Solana CLI."
  exit 1
fi

if [ ! -f "examples/cambrian-avs-demo/configs/avs-wallet.json" ]; then
  echo "🔧 First-time setup required. Running setup script..."
  
  cd examples/cambrian-avs-demo
  ./scripts/setup.sh
  cd ../..
else
  echo "✅ Setup already completed"
fi

# Start the AVS components
echo "🚀 Starting Cambrian AVS with Windexer..."
cd examples/cambrian-avs-demo
./scripts/start.sh
cd ../..

echo """
✅ Cambrian AVS demo is now running!

What would you like to do next?
1. Submit a proposal (examples/cambrian-avs-demo/scripts/submit-proposal.sh)
2. Stop the demo (examples/cambrian-avs-demo/scripts/stop.sh)

For more detailed instructions, refer to examples/cambrian-avs-demo/README.md
"""

echo "Press Ctrl+C to exit the demo"
tail -f examples/cambrian-avs-demo/logs/avs.log
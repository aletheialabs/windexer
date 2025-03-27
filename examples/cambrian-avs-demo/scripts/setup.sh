#!/bin/bash

# Setup script for Cambrian AVS Demo

echo "🚀 Setting up Cambrian AVS Demo..."

# Check if Solana CLI is installed
if ! command -v solana &> /dev/null; then
    echo "❌ Solana CLI not found. Please install it first."
    exit 1
fi

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "❌ Docker not found. Please install Docker and Docker Compose first."
    exit 1
fi

# Create config directory if it doesn't exist
mkdir -p configs

# Generate a new wallet if one doesn't exist
if [ ! -f "configs/avs-wallet.json" ]; then
    echo "🔑 Generating new wallet..."
    solana-keygen new -o configs/avs-wallet.json --no-bip39-passphrase

    # Fund the wallet on devnet
    PUBKEY=$(solana-keygen pubkey configs/avs-wallet.json)
    echo "💰 Funding wallet ${PUBKEY} on devnet..."
    solana airdrop 2 ${PUBKEY} --url https://api.devnet.solana.com
else
    echo "✅ Using existing wallet"
fi

# Create logs directory
mkdir -p logs
touch logs/avs.log

echo "✅ Setup complete! You can now run './scripts/start.sh' to start the demo." 
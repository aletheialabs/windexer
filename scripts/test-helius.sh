#!/bin/bash

# Test script for Helius API

# Get Helius API key from .env file
if [ -f .env ]; then
    HELIUS_API_KEY=$(grep HELIUS_API_KEY .env | cut -d'=' -f2)
else
    echo "Error: .env file not found"
    exit 1
fi

# Check if API key is set
if [ -z "$HELIUS_API_KEY" ]; then
    echo "Error: HELIUS_API_KEY not found in .env file"
    exit 1
fi

echo "Using Helius API Key: $HELIUS_API_KEY"

# Test getLatestBlockhash
echo -e "\nTesting getLatestBlockhash:"
curl -s "https://mainnet.helius-rpc.com/?api-key=$HELIUS_API_KEY" \
    -X POST \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"getLatestBlockhash","params":[]}' | jq

# Test getAccountInfo for a known account (SOL - System Program)
echo -e "\nTesting getAccountInfo for System Program:"
curl -s "https://mainnet.helius-rpc.com/?api-key=$HELIUS_API_KEY" \
    -X POST \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"getAccountInfo","params":["11111111111111111111111111111111",{"encoding":"base64"}]}' | jq

# Get recent transactions from Jupiter (a popular Solana DEX)
JUPITER_ADDRESS="JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB"
echo -e "\nGetting recent transactions for Jupiter..."
RECENT_TX=$(curl -s "https://mainnet.helius-rpc.com/?api-key=$HELIUS_API_KEY" \
    -X POST \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getSignaturesForAddress\",\"params\":[\"$JUPITER_ADDRESS\",{\"limit\":2}]}" | jq -r '.result[0].signature')

if [ -z "$RECENT_TX" ] || [ "$RECENT_TX" == "null" ]; then
    echo "Failed to get a recent transaction, using hardcoded value"
    RECENT_TX="4oBFNe4qV38QMF9pxUKTQib8w9MKzwcnzJkSvFMQjnJwN9YRqsS52LQEHoHJPsQK7yzXnQJs9JWm6wN2aHvA7jaN"
else
    echo "Found recent transaction: $RECENT_TX"
fi

# Test getTransaction with a recent transaction
echo -e "\nTesting getTransaction for a sample transaction:"
curl -s "https://mainnet.helius-rpc.com/?api-key=$HELIUS_API_KEY" \
    -X POST \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getTransaction\",\"params\":[\"$RECENT_TX\",{\"encoding\":\"json\",\"maxSupportedTransactionVersion\":0}]}" | jq 
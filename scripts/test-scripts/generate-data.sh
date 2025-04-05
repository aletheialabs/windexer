#!/bin/bash

set -e

# Configuration
VALIDATOR_PORT=8899
NUM_TRANSACTIONS=20
KEYPAIR_PATH="$HOME/.config/solana/id.json"

# Functions
create_keypair_if_needed() {
  if [ ! -f "$KEYPAIR_PATH" ]; then
    echo "Creating new Solana keypair..."
    solana-keygen new --force --no-passphrase -o "$KEYPAIR_PATH"
  fi
}

check_validator() {
  echo "Checking validator status..."
  # Try multiple times in case validator is still starting up
  for i in {1..5}; do
    if solana --url http://localhost:$VALIDATOR_PORT cluster-version &>/dev/null; then
      return 0
    fi
    echo "Waiting for validator... (attempt $i/5)"
    sleep 2
  done
  
  echo "Error: Solana validator is not running. Start it with 'make full-demo'"
  exit 1
}

airdrop_if_needed() {
  # Try multiple times as faucet might not be ready immediately
  for i in {1..3}; do
    balance=$(solana --url http://localhost:$VALIDATOR_PORT balance 2>/dev/null || echo "0")
    if (( $(echo "$balance < 1.0" | bc -l) )); then
      echo "Requesting airdrop of 2 SOL (attempt $i/3)..."
      if solana --url http://localhost:$VALIDATOR_PORT airdrop 2 &>/dev/null; then
        echo "Airdrop successful!"
        return 0
      fi
      sleep 2
    else
      echo "Current balance: $balance SOL"
      return 0
    fi
  done
  echo "Warning: Could not airdrop SOL. Continuing anyway..."
}

generate_transactions() {
  echo "Generating $NUM_TRANSACTIONS test transactions..."
  
  # Create recipient account
  recipient=$(solana-keygen new --force --no-passphrase --no-outfile | grep "pubkey" | cut -d ":" -f2 | xargs)
  
  for i in $(seq 1 $NUM_TRANSACTIONS); do
    amount=$(echo "scale=4; $RANDOM/1000000" | bc)
    echo "[$i/$NUM_TRANSACTIONS] Sending $amount SOL to $recipient"
    
    tx_sig=$(solana --url http://localhost:$VALIDATOR_PORT transfer --allow-unfunded-recipient \
      $recipient $amount --no-wait 2>/dev/null || echo "failed")
    
    if [ "$tx_sig" != "failed" ]; then
      echo "  Transaction sent: $tx_sig"
    else
      echo "  Failed to send transaction. Continuing..."
    fi
    
    # Small delay to spread out transactions
    sleep 0.5
  done
  
  echo "Waiting for transactions to finalize..."
  sleep 5
  
  # Check balance of recipient to confirm transfers
  echo "Recipient balance:"
  solana --url http://localhost:$VALIDATOR_PORT balance $recipient
}

# Main script
echo "=== wIndexer Data Generator ==="
create_keypair_if_needed
check_validator
airdrop_if_needed
generate_transactions
echo "Data generation complete!" 
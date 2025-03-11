#!/bin/bash

# Wait for services to start up
sleep 10

# Function to generate a random signature
random_sig() {
  cat /dev/urandom | tr -dc 'a-f0-9' | head -c 64
}

# Function to generate a random pubkey
random_pubkey() {
  cat /dev/urandom | tr -dc 'a-f0-9' | head -c 32
}

# Generate and send transaction data to indexer 1
for i in {1..20}; do
  echo "Generating transaction batch $i..."
  sig=$(random_sig)
  sender=$(random_pubkey)
  receiver=$(random_pubkey)
  amount=$((RANDOM % 1000))
  
  # Log the transaction we're creating
  echo "TX: $sender -> $receiver ($amount SOL) [sig: $sig]"
  
  # In a real scenario, we would use the Solana CLI to create actual transactions
  # But for this demo, we just wait to simulate transaction processing time
  sleep 2
done

echo "Sample data generation complete!" 
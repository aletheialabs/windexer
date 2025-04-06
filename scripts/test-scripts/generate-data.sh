#!/bin/bash

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration
VALIDATOR_HOST="localhost"
VALIDATOR_PORT=8899
NUM_TRANSACTIONS=20
KEYPAIR_PATH="$HOME/.config/solana/id.json"

echo -e "${BLUE}=== wIndexer Data Generator ===${NC}"

# Add Solana CLI if not installed
if ! command -v solana &> /dev/null; then
  echo -e "${YELLOW}Installing Solana CLI...${NC}"
  sh -c "$(curl -sSfL https://release.solana.com/v1.17.0/install)"
  export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
fi

# Functions
create_keypair_if_needed() {
  if [ ! -f "$KEYPAIR_PATH" ]; then
    echo -e "${YELLOW}Creating new Solana keypair...${NC}"
    solana-keygen new --force --no-passphrase -o "$KEYPAIR_PATH"
    echo -e "${GREEN}Keypair created at $KEYPAIR_PATH${NC}"
  fi
}

check_validator() {
  echo -e "${YELLOW}Checking validator status...${NC}"
  # Try multiple times in case validator is still starting up
  for i in {1..5}; do
    if solana --url http://$VALIDATOR_HOST:$VALIDATOR_PORT cluster-version &>/dev/null; then
      echo -e "${GREEN}Validator is running!${NC}"
      return 0
    fi
    echo -e "${YELLOW}Waiting for validator... (attempt $i/5)${NC}"
    sleep 2
  done

  echo -e "${RED}Error: Solana validator is not running. Start it with 'make run-validator-with-geyser'${NC}"
  exit 1
}

airdrop_if_needed() {
  # Try multiple times as faucet might not be ready immediately
  for i in {1..3}; do
    balance=$(solana --url http://$VALIDATOR_HOST:$VALIDATOR_PORT balance 2>/dev/null || echo "0")
    if (( $(echo "$balance < 1.0" | bc -l) )); then
      echo -e "${YELLOW}Requesting airdrop of 2 SOL (attempt $i/3)...${NC}"
      if solana --url http://$VALIDATOR_HOST:$VALIDATOR_PORT airdrop 2 &>/dev/null; then
        echo -e "${GREEN}Airdrop successful!${NC}"
        return 0
      fi
      sleep 2
    else
      echo -e "${GREEN}Current balance: $balance SOL${NC}"
      return 0
    fi
  done
  echo -e "${YELLOW}Warning: Could not airdrop SOL. Continuing anyway...${NC}"
}

generate_transactions() {
  echo -e "${BLUE}Generating $NUM_TRANSACTIONS test transactions...${NC}"

  # Create recipient account
  recipient=$(solana-keygen new --force --no-passphrase --no-outfile | grep "pubkey" | cut -d ":" -f2 | xargs)
  echo -e "${GREEN}Created recipient: $recipient${NC}"

  successful=0
  for i in $(seq 1 $NUM_TRANSACTIONS); do
    amount=$(echo "scale=4; $RANDOM/1000000" | bc)
    echo -e "${YELLOW}[$i/$NUM_TRANSACTIONS] Sending $amount SOL to $recipient${NC}"

    tx_sig=$(solana --url http://$VALIDATOR_HOST:$VALIDATOR_PORT transfer --allow-unfunded-recipient \
      $recipient $amount --no-wait 2>/dev/null || echo "failed")

    if [ "$tx_sig" != "failed" ]; then
      echo -e "  ${GREEN}Transaction sent: $tx_sig${NC}"
      ((successful++))
    else
      echo -e "  ${RED}Failed to send transaction. Continuing...${NC}"
    fi

    # Small delay to spread out transactions
    sleep 0.5
  done

  echo -e "${YELLOW}Waiting for transactions to finalize...${NC}"
  sleep 5

  # Check balance of recipient to confirm transfers
  recipient_balance=$(solana --url http://$VALIDATOR_HOST:$VALIDATOR_PORT balance $recipient 2>/dev/null || echo "unknown")
  echo -e "${GREEN}Recipient balance: $recipient_balance${NC}"
  echo -e "${BLUE}Successfully sent $successful out of $NUM_TRANSACTIONS transactions${NC}"
}

# Main script
create_keypair_if_needed
check_validator
airdrop_if_needed
generate_transactions
echo -e "${GREEN}Data generation complete!${NC}"
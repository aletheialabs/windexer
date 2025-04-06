#!/bin/bash

# Remove set -e to prevent silent failures and add trapping
# set -e

# Add error trapping for better diagnostics
trap 'echo "Error on line $LINENO. Exit code: $?" >&2' ERR

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration
VALIDATOR_HOST="localhost"
VALIDATOR_PORT=8999
NUM_TRANSACTIONS=20
KEYPAIR_PATH="../../examples/typescript/payer-keypair.json"

echo -e "${BLUE}=== wIndexer Data Generator ===${NC}"

# Enable debug output
set -x

# Check for required commands (returns 0 if found, 1 if not found)
check_command() {
  command -v $1 &> /dev/null
  return $?
}

# Add Solana CLI if not installed
if ! command -v solana &> /dev/null; then
  echo -e "${YELLOW}Installing Solana CLI...${NC}"
  sh -c "$(curl -sSfL https://release.solana.com/v1.17.0/install)"
  export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
fi

# Check if bc is available - do this silently
check_command bc
HAS_BC=$?
if [ $HAS_BC -ne 0 ]; then
  echo -e "${YELLOW}Warning: bc is not installed. Using alternative methods.${NC}"
fi

# Functions
create_keypair_if_needed() {
  if [ ! -f "$KEYPAIR_PATH" ]; then
    echo -e "${YELLOW}Creating new Solana keypair at $KEYPAIR_PATH...${NC}"
    solana-keygen new --force --no-passphrase -o "$KEYPAIR_PATH"
    echo -e "${GREEN}Keypair created at $KEYPAIR_PATH${NC}"
  else
    echo -e "${GREEN}Using existing keypair at $KEYPAIR_PATH${NC}"
  fi
}

check_validator() {
  echo -e "${YELLOW}Checking validator status...${NC}"
  # Try multiple times in case validator is still starting up
  for i in {1..5}; do
    echo "Attempt $i: Checking validator at http://$VALIDATOR_HOST:$VALIDATOR_PORT"
    if solana --url http://$VALIDATOR_HOST:$VALIDATOR_PORT cluster-version &>/dev/null; then
      echo -e "${GREEN}Validator is running!${NC}"
      return 0
    fi
    echo -e "${YELLOW}Waiting for validator... (attempt $i/5)${NC}"
    sleep 2
  done

  echo -e "${RED}Error: Solana validator is not running. Start it with 'make run-validator-with-geyser'${NC}"
  return 1  # Return error instead of exit to allow error handling
}

airdrop_if_needed() {
  echo -e "${YELLOW}Checking balance...${NC}"
  # Try multiple times as faucet might not be ready immediately
  for i in {1..3}; do
    balance=$(solana --keypair "$KEYPAIR_PATH" --url http://$VALIDATOR_HOST:$VALIDATOR_PORT balance 2>/dev/null || echo "0")
    echo "Current balance reported: $balance"
    
    if [ $HAS_BC -eq 0 ]; then
      need_airdrop=$(echo "$balance < 1.0" | bc -l)
    else
      # Alternative check without bc
      if [[ "$balance" == "0" || "$balance" == "0 SOL" ]]; then
        need_airdrop=1
      else
        # Extract the numeric part before "SOL" and compare
        balance_num=${balance%% SOL}
        # Simple check if it starts with "0." or is "0"
        if [[ "$balance_num" == "0"* && "$balance_num" != "0."[1-9]* ]]; then
          need_airdrop=1
        else
          need_airdrop=0
        fi
      fi
    fi
    
    echo "Need airdrop: $need_airdrop"
    if [ "$need_airdrop" == "1" ]; then
      echo -e "${YELLOW}Requesting airdrop of 2 SOL (attempt $i/3)...${NC}"
      solana --keypair "$KEYPAIR_PATH" --url http://$VALIDATOR_HOST:$VALIDATOR_PORT airdrop 2
      if [ $? -eq 0 ]; then
        echo -e "${GREEN}Airdrop successful!${NC}"
        return 0
      fi
      sleep 2
    else
      echo -e "${GREEN}Current balance: $balance${NC}"
      return 0
    fi
  done
  echo -e "${YELLOW}Warning: Could not airdrop SOL. Continuing anyway...${NC}"
}

generate_transactions() {
  echo -e "${BLUE}Generating $NUM_TRANSACTIONS test transactions...${NC}"

  # Create recipient keypair
  RECIPIENT_PATH=$(mktemp)
  echo -e "${YELLOW}Creating recipient keypair at $RECIPIENT_PATH...${NC}"
  solana-keygen new --force --no-passphrase -o "$RECIPIENT_PATH" &>/dev/null
  recipient=$(solana-keygen pubkey "$RECIPIENT_PATH")
  echo -e "${GREEN}Created recipient: $recipient${NC}"

  successful=0
  for i in $(seq 1 $NUM_TRANSACTIONS); do
    # Calculate a random small amount
    if [ $HAS_BC -eq 0 ]; then
      amount=$(echo "scale=4; $RANDOM/1000000" | bc)
    else
      # Alternative calculation without bc (smaller amounts)
      amount="0.000$(( RANDOM % 1000 + 1 ))"
    fi
    
    echo -e "${YELLOW}[$i/$NUM_TRANSACTIONS] Sending $amount SOL to $recipient${NC}"

    tx_sig=$(solana --keypair "$KEYPAIR_PATH" --url http://$VALIDATOR_HOST:$VALIDATOR_PORT \
      transfer --allow-unfunded-recipient $recipient $amount --no-wait 2>&1 || echo "failed")

    if [[ "$tx_sig" != *"failed"* && "$tx_sig" != *"Error"* ]]; then
      echo -e "  ${GREEN}Transaction sent: $tx_sig${NC}"
      ((successful++))
    else
      echo -e "  ${RED}Failed to send transaction: $tx_sig${NC}"
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
  
  # Clean up
  rm -f "$RECIPIENT_PATH"
}

# Main script
create_keypair_if_needed

# Call check_validator but handle errors
if ! check_validator; then
  echo -e "${RED}Validator check failed. Exiting.${NC}"
  exit 1
fi

airdrop_if_needed
generate_transactions

# Disable debug output
set +x

echo -e "${GREEN}Data generation complete!${NC}"
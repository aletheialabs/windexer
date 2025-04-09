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
VALIDATOR_PORT=8899
NUM_TRANSACTIONS=20
KEYPAIR_PATH="../../examples/typescript/payer-keypair.json"
TARGET_SIZE_GB=2
DATA_DIR="../../data"  # Directory where data is stored

echo -e "${BLUE}=== wIndexer Data Generator ===${NC}"
echo -e "${YELLOW}Target size: ${TARGET_SIZE_GB}GB${NC}"

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

calculate_sol() {
  local lamports=$1
  if [ $HAS_BC -eq 0 ]; then
    echo "scale=9; $lamports/1000000000" | bc -l
  else
    echo "$lamports" | awk '{printf "%.9f", $1/1000000000}'
  fi
}

# Function to get current data directory size in GB
get_data_size_gb() {
  if [ -d "$DATA_DIR" ]; then
    if [ $HAS_BC -eq 0 ]; then
      du -sb "$DATA_DIR" | awk '{printf "%.2f", $1/1024/1024/1024}'
    else
      du -sb "$DATA_DIR" | awk '{printf "%.2f", $1/1024/1024/1024}'
    fi
  else
    echo "0"
  fi
}

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
  return 1
}

airdrop_if_needed() {
  echo -e "${YELLOW}Checking balance...${NC}"
  # Try multiple times as faucet might not be ready immediately
  for i in {1..3}; do
    balance=$(solana --keypair "$KEYPAIR_PATH" --url http://$VALIDATOR_HOST:$VALIDATOR_PORT balance 2>/dev/null || echo "0")
    echo "Current balance reported: $balance"
    
    if [ $HAS_BC -eq 0 ]; then
      need_airdrop=$(echo "$balance < 10.0" | bc -l)
    else
      # Alternative check without bc
      if [[ "$balance" == "0" || "$balance" == "0 SOL" ]]; then
        need_airdrop=1
      else
        # Extract the numeric part before "SOL" and compare
        balance_num=${balance%% SOL}
        # Check if balance is less than 10 SOL
        if [[ "$balance_num" == "0"* || "$balance_num" < "10" ]]; then
          need_airdrop=1
        else
          need_airdrop=0
        fi
      fi
    fi
    
    echo "Need airdrop: $need_airdrop"
    if [ "$need_airdrop" == "1" ]; then
      echo -e "${YELLOW}Requesting airdrop of 10 SOL (attempt $i/3)...${NC}"
      solana --keypair "$KEYPAIR_PATH" --url http://$VALIDATOR_HOST:$VALIDATOR_PORT airdrop 10
      if [ $? -eq 0 ]; then
        echo -e "${GREEN}Airdrop successful!${NC}"
        # Wait for airdrop to be confirmed
        sleep 5
        return 0
      fi
      sleep 2
    else
      echo -e "${GREEN}Current balance: $balance${NC}"
      return 0
    fi
  done
  echo -e "${RED}Error: Could not airdrop SOL. Exiting.${NC}"
  exit 1
}

fund_recipient() {
    local recipient=$1
    local amount=$2
    local max_attempts=3
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        echo -e "${YELLOW}Attempt $attempt: Funding recipient with $amount SOL...${NC}"
        tx_sig=$(solana --keypair ../../examples/typescript/payer-keypair.json --url http://$VALIDATOR_HOST:$VALIDATOR_PORT transfer --allow-unfunded-recipient $recipient $amount --no-wait 2>&1)
        
        if [[ $tx_sig == *"failed"* ]]; then
            echo -e "${RED}Failed to send funding transaction on attempt $attempt${NC}"
            attempt=$((attempt + 1))
            sleep 2
            continue
        fi
        
        echo -e "${GREEN}Funding transaction sent: $tx_sig${NC}"
        echo -e "${YELLOW}Waiting for confirmation...${NC}"
        
        # Wait for confirmation with timeout
        timeout=30
        while [ $timeout -gt 0 ]; do
            # Check transaction status
            status=$(solana --url http://$VALIDATOR_HOST:$VALIDATOR_PORT confirm $tx_sig 2>&1)
            if [[ $status == *"Confirmed"* ]]; then
                echo -e "${GREEN}Funding transaction confirmed!${NC}"
                # Verify balance
                sleep 2
                balance=$(solana --url http://$VALIDATOR_HOST:$VALIDATOR_PORT balance $recipient 2>/dev/null || echo "0")
                if [ "$balance" != "0" ]; then
                    echo -e "${GREEN}Recipient balance verified: $balance SOL${NC}"
                    return 0
                fi
            fi
            sleep 1
            timeout=$((timeout - 1))
        done
        
        echo -e "${RED}Funding transaction timed out on attempt $attempt${NC}"
        attempt=$((attempt + 1))
    done
    
    echo -e "${RED}Failed to fund recipient after $max_attempts attempts${NC}"
    return 1
}

generate_transactions() {
  local batch_size=$1
  echo -e "${BLUE}Generating $batch_size test transactions...${NC}"

  # Get rent exemption amount
  echo -e "${YELLOW}Getting rent exemption amount...${NC}"
  RENT_EXEMPTION=$(solana --url http://$VALIDATOR_HOST:$VALIDATOR_PORT rent-exempt 0 | grep -o '[0-9]*')
  RENT_EXEMPTION_SOL=$(calculate_sol $RENT_EXEMPTION)
  echo -e "${GREEN}Rent exemption amount: $RENT_EXEMPTION_SOL SOL${NC}"

  # Create recipient keypair
  RECIPIENT_PATH=$(mktemp)
  echo -e "${YELLOW}Creating recipient keypair at $RECIPIENT_PATH...${NC}"
  solana-keygen new --force --no-passphrase -o "$RECIPIENT_PATH" &>/dev/null
  recipient=$(solana-keygen pubkey "$RECIPIENT_PATH")
  echo -e "${GREEN}Created recipient: $recipient${NC}"

  # Fund recipient with rent exemption
  if ! fund_recipient $recipient $RENT_EXEMPTION_SOL; then
    echo -e "${RED}Failed to fund recipient account. Exiting.${NC}"
    exit 1
  fi

  # Now send test transactions
  echo -e "${YELLOW}Sending test transactions...${NC}"
  successful_txns=0
  for i in $(seq 1 $batch_size); do
    amount=$(calculate_sol 100000)
    echo -e "${YELLOW}Sending transaction $i of $batch_size ($amount SOL)...${NC}"
    
    tx_sig=$(solana --keypair ../../examples/typescript/payer-keypair.json --url http://$VALIDATOR_HOST:$VALIDATOR_PORT transfer --allow-unfunded-recipient $recipient $amount --no-wait 2>&1)
    
    if [[ $tx_sig == *"failed"* ]]; then
        echo -e "${RED}Failed to send transaction $i${NC}"
        continue
    fi
    
    echo -e "${GREEN}Transaction sent: $tx_sig${NC}"
    sleep 1
    
    # Verify transaction
    if solana --url http://$VALIDATOR_HOST:$VALIDATOR_PORT confirm $tx_sig >/dev/null 2>&1; then
        successful_txns=$((successful_txns + 1))
        echo -e "${GREEN}Transaction $i confirmed successfully${NC}"
    else
        echo -e "${RED}Transaction $i failed to confirm${NC}"
    fi
  done

  echo -e "${GREEN}Successfully sent and confirmed $successful_txns out of $batch_size transactions${NC}"
  
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

# Create data directory if it doesn't exist
mkdir -p "$DATA_DIR"

# Main loop to generate data until target size is reached
while true; do
    current_size=$(get_data_size_gb)
    echo -e "${YELLOW}Current data size: ${current_size}GB / ${TARGET_SIZE_GB}GB${NC}"
    
    if [ $(echo "$current_size >= $TARGET_SIZE_GB" | bc -l) -eq 1 ]; then
        echo -e "${GREEN}Target size of ${TARGET_SIZE_GB}GB reached!${NC}"
        break
    fi
    
    # Generate a batch of transactions
    generate_transactions $NUM_TRANSACTIONS
    
    # Wait a bit before checking size again
    sleep 5
done

# Disable debug output
set +x

echo -e "${GREEN}Data generation complete!${NC}"
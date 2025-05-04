#!/bin/bash
set -euo pipefail

# Default values
NETWORK=${1:-devnet}
DECIMALS=${2:-9}
NETWORK_URL=""

# Set network URL based on selected network
case $NETWORK in
  devnet)
    NETWORK_URL="https://api.devnet.solana.com"
    ;;
  testnet)
    NETWORK_URL="https://api.testnet.solana.com"
    ;;
  mainnet)
    NETWORK_URL="https://api.mainnet-beta.solana.com"
    ;;
  local)
    NETWORK_URL="http://localhost:8899"
    ;;
  *)
    echo "Invalid network specified. Use 'devnet', 'testnet', 'mainnet', or 'local'"
    exit 1
    ;;
esac

echo "Connecting to $NETWORK..."
solana config set --url $NETWORK_URL

# Check if spl-token is installed
if ! command -v spl-token &> /dev/null; then
  echo "spl-token CLI is not installed. Installing it now..."
  cargo install spl-token-cli
  
  if ! command -v spl-token &> /dev/null; then
    echo "Failed to install spl-token CLI. Please install it manually."
    exit 1
  fi
fi

# Handle keypair for the mint
MINT_KEYPAIR="mint-keypair.json"
if [ -f "$MINT_KEYPAIR" ]; then
  echo "Using existing mint keypair from $MINT_KEYPAIR"
else
  # Create a new keypair for the mint
  echo "Creating new mint keypair..."
  solana-keygen new --no-bip39-passphrase -o $MINT_KEYPAIR
fi

# Extract mint address
MINT_ADDRESS=$(solana-keygen pubkey $MINT_KEYPAIR)
echo "Mint address: $MINT_ADDRESS"

# Owner is the current Solana CLI config keypair
OWNER_ADDRESS=$(solana address)
echo "Owner address: $OWNER_ADDRESS"

# Token-2022 program ID
TOKEN_2022_PROGRAM_ID="TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"

# Check if mint exists
echo "Checking if mint already exists..."
MINT_EXISTS=false
if spl-token --program-id $TOKEN_2022_PROGRAM_ID supply $MINT_ADDRESS 2>/dev/null; then
  echo "Mint already exists, skipping mint creation"
  MINT_EXISTS=true
else
  # Create the token mint using spl-token CLI
  echo "Creating confidential token mint with Token-2022 program..."
  spl-token --program-id $TOKEN_2022_PROGRAM_ID create-token \
    --enable-confidential-transfers auto \
    --decimals $DECIMALS \
    $MINT_KEYPAIR
fi

# Check if token account exists
TOKEN_ACCOUNT=""
TOKEN_ACCOUNT_EXISTS=false

# Try to find the token account
ACCOUNTS_OUTPUT=$(spl-token --program-id $TOKEN_2022_PROGRAM_ID accounts -v 2>/dev/null || echo "No accounts found")
if echo "$ACCOUNTS_OUTPUT" | grep -q "$MINT_ADDRESS"; then
  # Extract the token account address
  TOKEN_ACCOUNT=$(echo "$ACCOUNTS_OUTPUT" | grep "$MINT_ADDRESS" | awk '{print $1}')
  echo "Existing token account found: $TOKEN_ACCOUNT"
  TOKEN_ACCOUNT_EXISTS=true
else
  # Create a token account for the owner
  echo "Creating token account..."
  spl-token --program-id $TOKEN_2022_PROGRAM_ID create-account $MINT_ADDRESS
  
  # Get token account address
  TOKEN_ACCOUNT=$(spl-token --program-id $TOKEN_2022_PROGRAM_ID accounts -v | grep $MINT_ADDRESS | awk '{print $1}')
  echo "New token account created: $TOKEN_ACCOUNT"
fi

# Configure the token account for confidential transfers if it's not already
echo "Configuring token account for confidential transfers..."
spl-token --program-id $TOKEN_2022_PROGRAM_ID configure-confidential-transfer-account --address $TOKEN_ACCOUNT || {
  echo "Token account might already be configured for confidential transfers"
}

# Mint tokens only if we created a new mint
if [ "$MINT_EXISTS" = false ]; then
  # Mint some tokens to the owner's account
  echo "Minting initial tokens..."
  spl-token --program-id $TOKEN_2022_PROGRAM_ID mint $MINT_ADDRESS 1000
fi

# Save information to .env file
echo "export FLORIN_MINT_ADDRESS=$MINT_ADDRESS" > .env
echo "export SOLANA_TOKEN_ACCOUNT=$TOKEN_ACCOUNT" >> .env
echo "export SOLANA_NETWORK=$NETWORK" >> .env
echo "export SOLANA_URL=$NETWORK_URL" >> .env
echo "export OWNER_ADDRESS=$OWNER_ADDRESS" >> .env
echo "export TOKEN_2022_PROGRAM_ID=$TOKEN_2022_PROGRAM_ID" >> .env

echo "Configuration complete! Mint address saved to .env file."
echo "Use florin-core and florin-zk modules for client-side confidential transfer operations."
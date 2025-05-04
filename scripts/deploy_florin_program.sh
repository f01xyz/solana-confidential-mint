#!/bin/bash
set -euo pipefail

echo "Building Florin confidential transfer program (SBF)..."
cd florin-deploy
rustup override set 1.70.0
cargo build-sbf --sbf-out-dir ../target/deploy
cd ..

# Get the deployment target from argument (devnet, testnet, mainnet)
NETWORK=${1:-devnet}
NETWORK_URL=""

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

echo "Deploying to $NETWORK..."
PROGRAM_ID=$(solana program deploy \
  --keypair ~/.config/solana/id.json \
  --url $NETWORK_URL \
  --program-id florin-deploy/target/deploy/florin_deploy-keypair.json \
  target/deploy/florin_deploy.so)

echo "Program deployed with ID: $PROGRAM_ID"

# Save program ID for later use
echo "export FLORIN_PROGRAM_ID=$PROGRAM_ID" > .env
echo "export SOLANA_NETWORK=$NETWORK" >> .env
echo "export SOLANA_URL=$NETWORK_URL" >> .env

echo "Deployment complete! Program ID saved to .env file." 
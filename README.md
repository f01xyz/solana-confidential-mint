# Florin: Solana Confidential Token System

This repository contains the components for creating and managing confidential tokens on Solana using the Token-2022 program.

## Project Components

The project is divided into three main components:

1. **Token Deployment** - CLI scripts to create and manage confidential tokens on Solana
2. **florin-core** - Client-side library for token operations and business logic
3. **florin-zk** - Zero-knowledge proof generation for confidential transfers

## Quick Start

### Create a Confidential Token

```bash
# Deploy to devnet with 9 decimals (default)
./scripts/init_token.sh

# Deploy to a specific network with custom decimals
./scripts/init_token.sh devnet 6
./scripts/init_token.sh testnet 9
./scripts/init_token.sh mainnet 8
```

The script will:
- Create a token mint with confidential transfer capability
- Initialize a token account for confidential transfers
- Mint initial tokens to your account
- Save all relevant addresses to `.env`

### Using Your Confidential Token

After deployment, the script generates a `.env` file containing:

```
FLORIN_MINT_ADDRESS=<your-token-mint-address>
SOLANA_TOKEN_ACCOUNT=<your-token-account-address>
SOLANA_NETWORK=<network>
SOLANA_URL=<network-url>
OWNER_ADDRESS=<your-wallet-address>
TOKEN_2022_PROGRAM_ID=TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
```

You can then use these values in your applications.

## Confidential Token Operations

To perform basic operations with your confidential token, use the following commands:

```bash
# Check token supply
spl-token --program-id TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb supply <MINT_ADDRESS>

# Check token account balance
spl-token --program-id TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb balance <TOKEN_ACCOUNT>

# Deposit tokens to confidential balance
spl-token --program-id TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb deposit-confidential-tokens <MINT_ADDRESS> <AMOUNT>

# Apply pending balance
spl-token --program-id TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb apply-pending-balance --address <TOKEN_ACCOUNT>

# Transfer confidential tokens
spl-token --program-id TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb transfer <MINT_ADDRESS> <AMOUNT> <DESTINATION_ACCOUNT> --confidential

# Withdraw from confidential balance to public balance
spl-token --program-id TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb withdraw-confidential-tokens <MINT_ADDRESS> <AMOUNT>
```

## Architecture

Florin uses a layered approach to work with confidential tokens:

1. **Token-2022 On-chain Program** - Provides the confidential transfer extension on Solana
2. **CLI Layer** - Scripts for token deployment and basic management
3. **Client Libraries** - Provide higher-level abstractions for working with tokens:
   - `florin-core`: Business logic and standard token operations
   - `florin-zk`: Zero-knowledge proof generation and validation for confidential transfers

## Development Notes

- Confidential transfers require zero-knowledge proofs to be generated client-side
- All client-side operations should use the florin-core and florin-zk libraries
- For automated deployments, the `init_token.sh` script can be integrated into CI/CD pipelines

## Security Considerations

- Encryption keys for confidential transfers are separate from signing keys
- Users must apply pending balances before they can spend incoming confidential tokens
- Proofs for confidential transfers are validated on-chain
- Token mints can optionally have an auditor to decrypt transfer amounts

## License

[Apache License 2.0](LICENSE)

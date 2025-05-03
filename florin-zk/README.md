# Florin ZK

This project contains advanced Zero Knowledge functionality using Solana 2.0+ libraries.

## Purpose

This project is dedicated to:
- Generating and verifying real ZK proofs
- Using Solana 2.0+ ZK libraries (spl-token-confidential-transfer-proof-*)
- Potentially generating proofs off-chain for use with florin-core

## Architecture

This project is based on Solana 2.0.3+ and uses the following libraries:
- solana-sdk, solana-client, solana-program (v2.0.3+)
- spl-token, spl-token-2022, spl-token-client (updated for 2.0 compatibility)
- solana-zk-token-sdk (v2.0.3+)
- spl-token-confidential-transfer-proof-program
- spl-token-confidential-transfer-proof-api

## Usage

Build the project:
```
cargo build
```

Run the main CLI:
```
cargo run --bin florin-zk
```

## Related Projects

This project is part of a strategic split:
- **florin-core**: Solana 1.17-based core functionality
- **florin-zk** (this project): Solana 2.0+-based ZK proof generation and verification

## Integration Strategy

There are two potential paths for integration:
1. Export proofs from florin-zk and use them in florin-core
2. Eventually migrate all functionality to Solana 2.0+ when ready

The current focus is on developing and testing ZK functionality in isolation to avoid dependency conflicts. 
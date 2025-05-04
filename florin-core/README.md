# Florin Core

This project contains the core functionality for Confidential Tokens using Solana 1.18.222.

## Features

- Token minting
- Confidential transfers
- Internal ledger management
- CLI and webapp integration

## Architecture

This project is based on Solana 1.18.222 and uses the following libraries:
- solana-sdk, solana-client, solana-program (v1.18.222)
- spl-token, spl-token-2022, spl-token-client
- solana-zk-token-sdk (basic ZK functionality available in 1.17)

## Usage

Build the project:
```
cargo build
```

Run the main CLI:
```
cargo run --bin florin-core
```

Run test commands:
```
cargo run --bin test-create-mint
```

## Related Projects

This project is part of a strategic split:
- **florin-core** (this project): Solana 1.17-based core functionality
- **florin-zk**: Solana 2.0+-based ZK proof generation and verification

## Note

This project is locked to Solana 1.18.222 to maintain compatibility and stability. For advanced ZK functionality, refer to the florin-zk project. 
# Solana Confidential Mint

A Solana-based implementation of confidential token transfers using zero-knowledge proofs.

## Project Structure

```
solana-confidential-mint
├── florin-core/         # Solana 1.17-compatible token implementation
│   ├── src/
│   │   ├── lib/         # Core library code
│   │   │   ├── confidential_ops.rs  # Confidential token operations
│   │   │   └── mod.rs              # Library exports
│   │   └── main.rs      # Main CLI entry point
│   └── Cargo.toml       # Dependencies (Solana 1.17)
├── florin-zk/           # Solana 2.0+ ZK proof operations
│   ├── src/
│   │   ├── lib/
│   │   │   ├── proof_export.rs     # Proof export/import utilities
│   │   │   ├── zk_proofs.rs        # ZK proof generation & verification
│   │   │   └── mod.rs              # Library exports
│   │   └── main.rs      # CLI for proof generation
│   └── Cargo.toml       # Dependencies (Solana 2.0+)
└── Cargo.toml           # Workspace definition
```

## Components

- **florin-core**: Token implementation compatible with Solana 1.17, handling confidential token transfers
- **florin-zk**: Zero-knowledge proof generation and verification using Solana 2.0+ libraries

## Setup

### Prerequisites

- Rust 1.70+ and Cargo
- Solana CLI tools

### Building

```bash
# Build the entire workspace
cargo build

# Build just florin-core
cargo build -p florin-core

# Build just florin-zk
cargo build -p florin-zk
```

## Usage

### Confidential Mint Operations (florin-core)

```bash
cargo run -p florin-core -- [args]
```

### ZK Proof Generation (florin-zk)

```bash
# Generate a new ElGamal keypair
cargo run -p florin-zk -- genkeypair

# Generate a transfer proof
cargo run -p florin-zk -- transferproof --amount 1000 --output proof.json

# Generate a withdraw proof
cargo run -p florin-zk -- withdrawproof --amount 1000 --output withdraw.json

# Run a complete demo
cargo run -p florin-zk -- demo --amount 5000
```

## Integration Flow

1. Generate ZK proofs with `florin-zk`
2. Export proofs to JSON files
3. Import and use proofs with `florin-core` for confidential transactions

## License

MIT

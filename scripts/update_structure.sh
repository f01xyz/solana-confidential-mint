#!/bin/bash

# Ensure we're in the project root
cd "$(dirname "$0")/.." || exit 1

echo "# Solana Confidential Mint" > README.md
echo "" >> README.md
echo "A Solana-based implementation of confidential token transfers using zero-knowledge proofs." >> README.md
echo "" >> README.md

echo "## Project Structure" >> README.md
echo "" >> README.md
echo "\`\`\`" >> README.md
echo "solana-confidential-mint" >> README.md
echo "├── florin-core/         # Solana 1.17-compatible token implementation" >> README.md
echo "│   ├── src/" >> README.md
echo "│   │   ├── lib/         # Core library code" >> README.md
echo "│   │   │   ├── confidential_ops.rs  # Confidential token operations" >> README.md
echo "│   │   │   └── mod.rs              # Library exports" >> README.md
echo "│   │   └── main.rs      # Main CLI entry point" >> README.md
echo "│   └── Cargo.toml       # Dependencies (Solana 1.17)" >> README.md
echo "├── florin-zk/           # Solana 2.0+ ZK proof operations" >> README.md
echo "│   ├── src/" >> README.md
echo "│   │   ├── lib/" >> README.md
echo "│   │   │   ├── proof_export.rs     # Proof export/import utilities" >> README.md
echo "│   │   │   ├── zk_proofs.rs        # ZK proof generation & verification" >> README.md
echo "│   │   │   └── mod.rs              # Library exports" >> README.md
echo "│   │   └── main.rs      # CLI for proof generation" >> README.md
echo "│   └── Cargo.toml       # Dependencies (Solana 2.0+)" >> README.md
echo "└── Cargo.toml           # Workspace definition" >> README.md
echo "\`\`\`" >> README.md
echo "" >> README.md

echo "## Components" >> README.md
echo "" >> README.md
echo "- **florin-core**: Token implementation compatible with Solana 1.17, handling confidential token transfers" >> README.md
echo "- **florin-zk**: Zero-knowledge proof generation and verification using Solana 2.0+ libraries" >> README.md
echo "" >> README.md

echo "## Setup" >> README.md
echo "" >> README.md
echo "### Prerequisites" >> README.md
echo "" >> README.md
echo "- Rust 1.70+ and Cargo" >> README.md
echo "- Solana CLI tools" >> README.md
echo "" >> README.md

echo "### Building" >> README.md
echo "" >> README.md
echo "\`\`\`bash" >> README.md
echo "# Build the entire workspace" >> README.md
echo "cargo build" >> README.md
echo "" >> README.md
echo "# Build just florin-core" >> README.md
echo "cargo build -p florin-core" >> README.md
echo "" >> README.md
echo "# Build just florin-zk" >> README.md
echo "cargo build -p florin-zk" >> README.md
echo "\`\`\`" >> README.md
echo "" >> README.md

echo "## Usage" >> README.md
echo "" >> README.md
echo "### Confidential Mint Operations (florin-core)" >> README.md
echo "" >> README.md
echo "\`\`\`bash" >> README.md
echo "cargo run -p florin-core -- [args]" >> README.md
echo "\`\`\`" >> README.md
echo "" >> README.md

echo "### ZK Proof Generation (florin-zk)" >> README.md
echo "" >> README.md
echo "\`\`\`bash" >> README.md
echo "# Generate a new ElGamal keypair" >> README.md
echo "cargo run -p florin-zk -- genkeypair" >> README.md
echo "" >> README.md
echo "# Generate a transfer proof" >> README.md
echo "cargo run -p florin-zk -- transferproof --amount 1000 --output proof.json" >> README.md
echo "" >> README.md
echo "# Generate a withdraw proof" >> README.md
echo "cargo run -p florin-zk -- withdrawproof --amount 1000 --output withdraw.json" >> README.md
echo "" >> README.md
echo "# Run a complete demo" >> README.md
echo "cargo run -p florin-zk -- demo --amount 5000" >> README.md
echo "\`\`\`" >> README.md
echo "" >> README.md

echo "## Integration Flow" >> README.md
echo "" >> README.md
echo "1. Generate ZK proofs with \`florin-zk\`" >> README.md
echo "2. Export proofs to JSON files" >> README.md
echo "3. Import and use proofs with \`florin-core\` for confidential transactions" >> README.md
echo "" >> README.md

echo "## License" >> README.md
echo "" >> README.md
echo "MIT" >> README.md

echo "Project structure updated and documented in README.md"

# Make the script executable
chmod +x scripts/update_structure.sh 
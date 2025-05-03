# Solana Confidential Mint Projects

This repository contains two separate projects for Solana confidential token functionality, strategically split to avoid version conflicts.

## Project Structure

### florin-core
- Based on Solana 1.17.6
- Working Confidential Token flow
- CLI, webapp integration
- Purpose: MVP, token minting, transfers, internal ledger

### florin-zk
- Based on Solana 2.0.3+
- Uses advanced ZK libraries:
  - solana-zk-token-sdk (2.0.3+)
  - spl-token-confidential-transfer-proof-*
- Dedicated to:
  - Generating and verifying real ZK proofs
  - Potentially generating proofs off-chain for use with florin-core

## Rationale for the Split

- Mixing Solana 1.17 and 2.0 in one repo leads to dependency conflicts
- ZK libraries (proof-gen, decrypt, AES) require Solana v2.0+
- The 1.17-based implementation is stable and works for basic confidential token operations

## Development Workflow

1. For basic confidential token functionality, work in `florin-core/`
2. For advanced ZK proof generation and verification, work in `florin-zk/`
3. Integration options:
   - Export proofs from florin-zk and use them in florin-core
   - Eventually migrate everything to Solana 2.0+ when ready

## Getting Started

See the individual README files in each project directory for specific instructions:
- [florin-core/README.md](florin-core/README.md)
- [florin-zk/README.md](florin-zk/README.md) 
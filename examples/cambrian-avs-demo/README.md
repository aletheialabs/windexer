# Windexer Cambrian AVS Demo

This demo showcases the integration between Windexer (a Solana data indexing solution) and Cambrian AVS (Actively Validated Service) for secure, validated operations on the Solana blockchain.

## Overview

This demo uses the actual Rust implementation of the Windexer-Cambrian integration from the `windexer-jito-staking` crate. It demonstrates how Windexer can be used with Cambrian to provide validated indexing services.

## Prerequisites

- Docker and Docker Compose
- Rust (for building the code)
- Solana CLI tools
- An internet connection for Solana devnet access

## Getting Started

1. Setup the environment:
   ```bash
   ./scripts/setup.sh
   ```

2. Start the demo:
   ```bash
   ./scripts/start.sh
   ```

3. Submit a proposal:
   ```bash
   ./scripts/submit-proposal.sh basic
   ```
   
   Available payload types:
   - `basic` - Simple demonstration
   - `nft` - NFT-related operations
   - `mpl` - Metaplex operations
   - `spl` - SPL token operations
   - `update-nft` - Update NFT authority

4. View logs:
   ```bash
   docker-compose logs -f
   ```

5. Stop the demo:
   ```bash
   ./scripts/stop.sh
   ```

## Architecture

This demo includes:

1. **Windexer AVS Service** - Uses the Rust implementation from `crates/windexer-jito-staking/src/bin/avs.rs`
2. **Windexer Indexer** - Processes and indexes Solana blockchain data
3. **Cambrian Integration** - From `crates/windexer-jito-staking/src/cambrian/`

## Troubleshooting

- If the build fails, try building manually with `docker-compose build`
- Check the logs in `logs/avs.log` for detailed information
- Ensure ports 3000, 3001, and 9000 are available on your machine 
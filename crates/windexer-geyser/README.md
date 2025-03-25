# windexer-geyser

A Solana Geyser plugin for streaming blockchain data to the wIndexer network.

## Overview

This crate implements Solana's Geyser plugin interface to stream account updates, transactions, blocks, and other on-chain data to indexing nodes in the wIndexer network. It serves as the bridge between Solana validators and the wIndexer ecosystem.

## Features

Stream account updates in real-time
Capture and forward transactions
Process block and slot data
Configurable filtering for accounts and transactions
Built-in metrics collection
Installation
1. Build the plugin:

```bash
cargo build --package windexer-geyser --release
```

2. Copy the compiled library to your Solana validator:

```bash
cp target/release/libwindexer_geyser.so /path/to/solana/validator/plugins/
```

3. Create a configuration file:

```bash
cp config/geyser/windexer-geyser-config.json /path/to/solana/validator/config/
```

4. Restart your Solana validator to load the new plugin.

## Configuration

The plugin is configured through a JSON file. Example configuration:

```json
{
  "libpath": "/path/to/libwindexer_geyser.so",
  "network": {
    "node_id": "windexer-node",
    "listen_addr": "127.0.0.1:8900",
    "rpc_addr": "127.0.0.1:8901",
    "bootstrap_peers": ["other-node:9000"],
    "data_dir": "./data/geyser",
    "solana_rpc_url": "http://localhost:8899"
  },
  "accounts_selector": {
    "accounts": ["*"],
    "owners": null
  },
  "transaction_selector": {
    "mentions": ["*"],
    "include_votes": false
  },
  "thread_count": 4,
  "batch_size": 100,
  "use_mmap": true,
  "panic_on_error": false
}
```

## Usage

Start your Solana validator with the plugin:

```bash
solana-validator \
  --geyser-plugin-config /path/to/windexer-geyser-config.json \
  ... other validator options ...
```

Or for testing:

```bash
solana-test-validator \
  --geyser-plugin-config config/geyser/windexer-geyser-config.json \
  --reset
```

## Documentation

For more detailed documentation, run:

```bash
cargo doc --package windexer-geyser --open
```
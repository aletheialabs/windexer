# wIndexer Examples

This crate contains example implementations for the wIndexer project.

## Examples

### Local Data Generator (`local-gen`)

This example demonstrates how to generate test data using a local validator and Geyser plugin. It creates mock accounts, transactions, and blocks, and stores them using the wIndexer store.

#### Features

- Generates mock Solana accounts, transactions, and blocks
- Configurable number of items to generate
- Real-time metrics tracking
- Local storage using wIndexer store
- Graceful shutdown handling

#### Usage

1. First, start a local Solana validator with the Geyser plugin:

```bash
# Build the Geyser plugin
make build-geyser

# Start the validator with Geyser plugin
make run-validator-with-geyser
```

2. In a new terminal, run the local generator:

```bash
cargo run --bin local-gen
```

#### Command Line Options

- `--base-port`: Base port for P2P communication (default: 9000)
- `--bootstrap-peers`: Comma-separated list of bootstrap peer addresses
- `--solana-rpc`: Solana RPC URL (default: http://localhost:8899)
- `--data-dir`: Directory for storing data (default: ./data)
- `--metrics-interval-seconds`: Interval for metrics logging (default: 30)
- `--num-accounts`: Number of accounts to generate (default: 100)
- `--num-transactions`: Number of transactions to generate (default: 1000)
- `--num-blocks`: Number of blocks to generate (default: 100)

#### Example with Custom Options

```bash
cargo run --bin local-gen \
    --base-port 9000 \
    --data-dir ./test-data \
    --num-accounts 200 \
    --num-transactions 2000 \
    --num-blocks 200 \
    --metrics-interval-seconds 10
```

#### Output

The generator will output metrics every 30 seconds (or as configured) showing:
- Number of accounts generated and rate
- Number of transactions generated and rate
- Number of blocks generated and rate
- Last processed slot

#### Data Storage

Generated data is stored in the specified data directory under `local_gen/store/`. The store uses a combination of memory cache and disk storage for efficient data access.

#### Shutdown

Press Ctrl+C to gracefully shut down the generator. The shutdown process will:
1. Stop accepting new data
2. Complete processing of in-flight data
3. Close the store
4. Exit cleanly

### Other Examples

- `node`: Example of running a wIndexer node
- `indexer`: Example of running a wIndexer indexer 
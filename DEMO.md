# Windexer Demo Guide

## Introduction

Windexer is a scalable indexing system for Solana blockchain data. It consists of several components that work together to capture, process, and serve blockchain data through simple HTTP APIs. This demo guide will walk you through the complete process of running the Windexer system locally, generating test transactions, and querying the indexed data.

By following this guide, you'll gain hands-on experience with:
- Running a local Solana validator with the Geyser plugin
- Setting up a Windexer indexer to process blockchain data
- Starting the API server to expose indexed data
- Generating test transactions
- Querying indexed data through various methods
- Exporting data in various formats (JSON, CSV, Parquet)

Let's get started!

## System Architecture

Windexer consists of several components that work together:

```
                       ┌─────────────────┐
                       │                 │
                       │  Solana         │
                       │  Validator      │
                       │                 │
                       └────────┬────────┘
                                │
                                │ Geyser Plugin
                                │
                       ┌────────▼────────┐
                       │                 │
                       │  Windexer       │
                       │  Indexer        │
                       │                 │
                       └────────┬────────┘
                                │
                                │ Indexed Data
                                │
                       ┌────────▼────────┐
                       │                 │
┌─────────────┐        │  Windexer       │        ┌─────────────┐
│             │◄───────┤  API Server     ├───────►│             │
│  TypeScript │        │                 │        │  Curl/HTTP  │
│  Client     │        └─────────────────┘        │  Requests   │
│             │                                   │             │
└─────────────┘                                   └─────────────┘
```

- **Solana Validator**: Runs a local Solana blockchain
- **Geyser Plugin**: Streams blockchain data from the validator to external systems
- **Windexer Indexer**: Processes and indexes blockchain data 
- **Windexer API**: Serves indexed data through HTTP endpoints
- **Clients**: Query the API to retrieve indexed blockchain data

## Important Paths and Configuration

- **Geyser Plugin Config**: `config/geyser/windexer-geyser-config.json`
- **Validator Data**: `test-ledger/`
- **Indexer Data**: `data/indexer_0/`
- **TypeScript Examples**: `examples/typescript/`
- **Default Payer Keypair**: `examples/typescript/payer-keypair.json`
- **Exported Data**: `data/exports/`

## Prerequisites

- Rust and Cargo installed
- Solana CLI tools installed
- Bun or Node.js installed (for TypeScript examples)

## Step 1: Build the Project

First, build all the required crates and binaries:

```bash
# From the project root
cargo build --workspace
```

## Step 2: Start the Solana Validator with Geyser Plugin

The Geyser plugin allows the validator to push data to our indexer:

```bash
# Run a local Solana validator with the Geyser plugin
make run-validator-with-geyser
```

Wait for the validator to start. You should see output indicating the validator is processing blocks.

## Step 3: Start the Indexer

Start the indexer that will process transactions from the Geyser plugin:

```bash
# Run the indexer with transaction indexing enabled
RUST_LOG=debug target/debug/indexer --index 1 --base-port 10001 --solana-rpc 'http://localhost:8899' --data-dir ./data --index-types accounts,transactions,blocks --log-level debug --metrics-interval-seconds 30
```
> to check if validator is running, you can use `curl -s -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' http://localhost:8899`

The indexer will start and connect to the validator through the Geyser plugin.

## Step 4: Generate Test Data

Generate test transactions on the Solana validator:

```bash
# Generate test transactions
make generate-data
```

This will create several transactions by transferring small amounts of SOL between accounts.

## Step 5: Query and Export Data

You can now query and export the indexed data using various methods:

### Real-time Indexing and Export

The real-time indexer continuously monitors for new transactions and exports them in your chosen format:

```bash
# Export transactions, accounts, and stats in JSON format with 1-second interval
bun run examples/typescript/index-realtime-data.ts --transactions --accounts --stats --format json --interval 1
```

Available export options:
- `--transactions`: Export transaction data
- `--accounts`: Export account data
- `--stats`: Export indexer statistics
- `--format`: Export format (json, csv, or parquet)
- `--interval`: Export interval in seconds

### Query All Data

To query and export all existing data:

```bash
# Query and export all data
bun run examples/typescript/query-all-data.ts
```

### Using the API directly:

```bash
# Check API status
curl http://localhost:10001/api/health

# Query transactions
curl http://localhost:10001/api/transactions | jq

# Query accounts
curl http://localhost:10001/api/accounts | jq
```

## Step 6: Running the Full Scenario

You can run the complete setup with a single command:

```bash
# Run the full scenario (validator + indexer + data generation + real-time indexing)
make run-full-scenario
```

This will:
1. Stop any existing processes
2. Start the validator with Geyser plugin
3. Start the indexer
4. Generate test data
5. Start the real-time indexer

## Export Data Formats

The exported data is saved in the `data/exports/` directory with timestamped filenames. For example:

- `transactions_2025-04-09T09-01-16-169Z.json`
- `accounts_2025-04-09T09-01-16-169Z.csv`
- `stats_2025-04-09T09-01-16-169Z.parquet`

### Transaction Data

Transaction exports include:
- `signature`: Transaction signature
- `slot`: Slot number
- `success`: Success status
- `fee`: Transaction fee
- `accounts`: Array of account addresses involved
- `timestamp`: ISO timestamp
- `blockTime`: Block timestamp

### Account Data

Account exports include:
- `pubkey`: Account public key
- `lamports`: Account balance
- `owner`: Account owner
- `executable`: Whether the account is executable
- `rentEpoch`: Rent epoch
- `data`: Account data (if available)

### Indexer Stats

Stats exports include:
- `totalTransactions`: Total number of transactions processed
- `lastProcessedSlot`: Latest slot that has been processed
- `startTime`: Indexer start time
- `lastExportTime`: Time of the last export

## Port Configuration

| Component | Default Port | Configuration |
|-----------|--------------|--------------|
| Solana Validator | 8899 | RPC port in `make run-validator-with-geyser` |
| Indexer Base Port | 10001 | Configured with `--base-port` flag |
| Indexer RPC Port | 11001 | Base port + 1000 |
| Indexer API | 10001 | Same as base port |

## Troubleshooting

### API Connection Issues

If you get "API not found" errors:
1. Ensure the indexer is running on the correct port (10001)
2. Check if the indexer has started successfully with no errors
3. Try using the health endpoint: `curl http://localhost:10001/api/health`

### Missing Transactions

If transactions aren't showing up in the index:
1. Check that the validator with Geyser plugin is running
2. Verify the indexer is running with the `--index-types transactions` flag
3. Check the indexer logs for any error messages

### Port Conflicts

If you see "Address already in use" errors:
1. Find and stop processes using the conflicting ports: `lsof -i :10001`
2. Try using different ports for the indexer: `--base-port 13001`

## Clean Up

When you're finished, you can stop all processes:

```bash
# Stop the validator
make kill-validator

# Stop the indexer
make stop-indexer
```

## Full System Reset

To reset the entire system and start fresh:

```bash
# Clean up all data and stop processes
make clean
```

This will stop all processes and remove generated data directories.

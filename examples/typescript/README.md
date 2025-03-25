# wIndexer TypeScript Examples

Examples demonstrating how to interact with wIndexer using TypeScript.

## Overview

This directory contains TypeScript examples that show how to:
- Query data from wIndexer nodes
- Subscribe to updates via WebSocket
- Query the Solana blockchain directly
- Generate test transactions

## Setup

1. Install dependencies:
```bash
npm install
```
2. Make sure your wIndexer network is running. Start with:
```bash
cd ../..  # Navigate to project root
make run-validator-with-geyser
# In another terminal
make run-node-1
# In another terminal
make run-indexer-1
```

## Examples

1. Query wIndexer API

Demonstrates how to query the wIndexer HTTP API:
```bash
npm run query-windexer
```

2. WebSocket Subscription

Shows how to subscribe to real-time updates:
```bash
npm run websocket
```

3. Query Solana

Queries the Solana blockchain directly for comparison:
```bash
npm run query-solana
```

4. Generate Test Data

Creates test transactions to observe indexing:
```bash
npm run generate-data
```

## Configuration

The examples assume that:
- Solana validator is running on port 8899
- wIndexer node is running on port 9000
- wIndexer indexer is running on port 10001

You can modify these in the individual example files if needed.

## Further Reading

For more information on wIndexer, see the [wIndexer Documentation](https://docs.windexer.com).
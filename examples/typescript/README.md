# wIndexer TypeScript Examples

This directory contains TypeScript examples for interacting with wIndexer and Solana.

## Prerequisites

- [Node.js](https://nodejs.org/) (v16 or later)
- [npm](https://www.npmjs.com/) or [bun](https://bun.sh/)
- Local Solana validator running with Geyser plugin
- wIndexer node running

## Setup

1. Install dependencies:
   ```
   npm install
   ```
   or with bun:
   ```
   bun install
   ```

2. Make sure your services are running:
   ```
   # Terminal 1: Run Solana validator with Geyser plugin
   cd /home/vivek/projects/aletheia/windexer/windexer
   make run-validator-with-geyser
   
   # Terminal 2: Run wIndexer node
   cd /home/vivek/projects/aletheia/windexer/windexer
   make run-node-0
   ```

## Available Examples

### Generate Test Transactions

The simplest way to generate test transactions:

```
npm run simple-tx
```

This script sends simple transactions without requiring WebSocket confirmation.

### Query Solana Validator

```
npm run query-solana
```

This script queries basic information from your local Solana validator.

### Subscribe to Solana Events

```
npm run websocket
```

This script subscribes to Solana events via WebSockets.

### Query wIndexer API

```
npm run query-windexer
```

This script queries the wIndexer API for indexed transactions and accounts.

## Troubleshooting

### WebSocket Errors

If you encounter WebSocket errors with `generate-data.ts`, try using the simpler 
`simple-tx.ts` script which doesn't rely on WebSockets for confirmation:

```
npm run simple-tx
```

### Airdrop Issues

If automatic airdrops fail, you can use the Solana CLI to request an airdrop:

```
solana airdrop 2 YOUR_WALLET_ADDRESS --url http://localhost:8899
```

### Error: "Cannot find module '@solana/web3.js'"

Run `npm install` to ensure all dependencies are installed correctly.
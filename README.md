<div align="center">
  <h1>wIndexer</h1>
  <p>A distributed blockchain indexing solution for Solana</p>
  
  [![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](LICENSE)
</div>

## Overview

wIndexer is a high-performance, distributed indexing solution for the Solana blockchain. It enables developers to efficiently index, query, and monitor blockchain data through a decentralized peer-to-peer network of indexing nodes.

### Key Features

- **Real-time data indexing** via Solana's Geyser plugin interface
- **Distributed P2P architecture** for high availability and scalability
- **HTTP and WebSocket API** for querying indexed data
- **TypeScript SDK** for seamless integration with web applications
- **Performant storage layer** for fast data retrieval
- **Jito MEV integration** for tip routing and restaking capabilities

## Architecture

wIndexer consists of several modular components:

- **Geyser Plugin**: Connects directly to Solana validators to stream real-time data
- **Node Network**: P2P network for data propagation and redundancy
- **Indexers**: Specialized nodes that index and serve data via API
- **Client SDK**: Libraries for interacting with wIndexer services

## Getting Started

### Prerequisites

- Rust 1.70+ and Cargo
- Node.js 16+ and npm/yarn (for TypeScript examples)
- Solana CLI tools

### Quick Start and Testing

1. Clone the repository:

```bash
git clone https://github.com/aletheia-labs/windexer.git
cd windexer
```

2. Build the project:

```bash
cargo build --workspace
```

3. Run the project:

```bash
make run-validator-with-geyser
```
In another terminal, run:

```bash
make run-node-1
```

In another terminal, run:

```bash
make run-indexer-1
```

4. Generate test data:

```bash
cd examples/typescript
npm install
npm run generate-data
```

5. Query the indexed data:

```bash
npm run query-windexer
```

### Running with Docker
We also provide Docker images for easy deployment.

```bash
cd deployment/docker
docker compose up -d
```

## Documentation

- [Architecture Overview](docs/architecture.md)
- [Geyser Setup](docs/geyser-setup.md)
- [API Reference](docs/api-reference.md)
- [TypeScript SDK](docs/typescript-sdk.md)

## Examples

Check out the [examples](examples/) directory for code samples:

- [TypeScript Examples](examples/typescript/): Query indexed data, subscribe to WebSocket events, and generate test transactions
- [Python Examples](examples/python/): Sample scripts for interacting with the API

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under GPL-3.0-or-later. See the [LICENSE](LICENSE) file for details.

## Features

- Fast and efficient data ingestion from Solana blockchain
- Multiple storage backend options (RocksDB, Parquet, PostgreSQL)
- Configurable accounts and transaction filtering
- Hot/cold storage separation for optimal performance
- Robust metrics and monitoring

## Storage Options

wIndexer now supports multiple storage backends to suit different use cases:

### RocksDB (Default)

The default storage option, optimized for high-throughput write operations.

- **Pros**: Fast writes, good for hot storage
- **Cons**: Limited query capabilities
- **Best for**: Real-time data ingestion, recent data access

### Apache Parquet

Columnar storage format optimal for analytics workloads.

- **Pros**: Excellent compression (3-4x better than JSON), high query performance
- **Cons**: Append-only, not suited for frequent updates
- **Best for**: Analytics, cold storage, historical data

### PostgreSQL

Relational database with rich query capabilities.

- **Pros**: SQL queries, excellent for complex analytics, point-in-time recovery
- **Cons**: Higher resource usage, slower writes than specialized options
- **Best for**: Complex queries, local development, transaction analysis

## Hot/Cold Storage Architecture

wIndexer supports a hot/cold storage architecture:

1. **Hot Storage**: Recent data stored in RocksDB for fast access
2. **Cold Storage**: Historical data archived in Parquet or PostgreSQL

This approach provides the best of both worlds:
- Fast write performance during data ingestion
- Excellent query performance for analytics on historical data

## Configuration

Example configuration with multiple storage options:

```json
{
  "storage": {
    "storage_type": "rocksdb",     // Primary storage type (rocksdb, parquet, postgres)
    "rocksdb_path": "/path/to/rocksdb",
    "hot_cold_separation": true,   // Enable hot/cold storage separation
    "parquet": {
      "directory": "/path/to/parquet",
      "max_file_size_mb": 256,
      "compression_enabled": true,
      "partition_by_slot": true    // Create separate files by slot ranges
    },
    "postgres": {
      "connection_string": "postgres://user:password@localhost:5432/windexer",
      "create_tables": true,
      "batch_size": 1000,
      "max_connections": 10
    }
  }
}
```

## Installation

[Installation instructions...]

## Usage

[Usage instructions...]




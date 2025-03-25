# windexer-network

P2P networking layer for the wIndexer distributed system.

## Overview
This crate provides the peer-to-peer networking infrastructure that powers the wIndexer distributed system. It handles node discovery, data propagation, and consensus across the network.

## Features
- **Peer discovery**: Automatic peer finding using mDNS and bootstrap nodes
- **Message propagation**: Efficient gossipsub-based message propagation
- **Data synchronization**: Sync state between peers
- **Consensus**: Consensus protocol for distributed decision making
- **Network metrics**: Built-in monitoring of network performance

## Usage

Add this crate as a dependency in your `Cargo.toml`:

```toml
[dependencies]
windexer-network = { path = "../path/to/windexer-network" }
```
### Example

```rust
use windexer_network::Node;
use windexer_common::config::NodeConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configure the node
    let config = NodeConfig {
        node_id: "my-node".to_string(),
        listen_addr: "127.0.0.1:9000".parse()?,
        rpc_addr: "127.0.0.1:9001".parse()?,
        bootstrap_peers: vec!["127.0.0.1:9002".to_string()],
        data_dir: "./data".to_string(),
        solana_rpc_url: "http://localhost:8899".to_string(),
        // ... other config
    };

    // Create and start the node
    let (mut node, shutdown_tx) = Node::create_simple(config).await?;
    node.start().await?;

    // ... your application logic ...

    // To shut down:
    shutdown_tx.send(()).await?;
    
    Ok(())
}
```

## Running a wIndexer Node

For more detailed documentation, run:

```bash
cargo run --bin node -- --index 0 --base-port 9000 --enable-tip-route
```
## Documentation
For more detailed documentation, run:

```bash
cargo doc --package windexer-network --open
```
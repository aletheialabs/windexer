use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;
use windexer_common::config::NodeConfig;
use windexer_network::Node;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Node ID
    #[arg(long, default_value = "windexer-node")]
    node_id: String,

    /// Listen address port
    #[arg(long, default_value = "9000")]
    port: u16,

    /// RPC port
    #[arg(long, default_value = "8899")]
    rpc_port: u16,

    /// Bootstrap peers
    #[arg(long, value_delimiter = ',')]
    bootstrap_peers: Vec<String>,

    /// Data directory
    #[arg(long)]
    data_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    windexer_network::init_logging();

    // Parse command line arguments
    let args = Args::parse();
    
    // Create the NodeConfig using the correct constructor
    let mut config = NodeConfig::new_local(
        args.node_id,
        args.port,
        args.rpc_port,
        args.bootstrap_peers,
    );
    
    if let Some(data_dir) = args.data_dir {
        config.data_dir = data_dir.to_string_lossy().to_string();
    }

    info!("Starting windexer network node with config: {:?}", config);
    
    // Create and start the node
    let (mut node, _shutdown_tx) = Node::create_simple(config).await?;
    node.start().await?;
    
    Ok(())
} 
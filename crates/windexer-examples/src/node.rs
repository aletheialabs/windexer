// examples/node.rs
use {
    anyhow::Result,
    clap::Parser,
    solana_sdk::signer::keypair::Keypair,
    std::{
        sync::Arc,
        sync::atomic::{AtomicBool, Ordering},
        time::Duration,
    },
    tokio,
    tracing::{info, warn},
    tracing_subscriber::EnvFilter,
    windexer_common::{
        config::{NodeType, PublisherNodeConfig, RelayerNodeConfig, NodeConfig},
        crypto::SerializableKeypair,

    },
    windexer_jito_staking::StakingConfig,
    windexer_network::Node,
    ctrlc,
};

#[derive(Parser, Debug)]
#[clap(
    version, 
    about = "wIndexer node for Jito integration",
    long_about = "Runs a wIndexer node that connects to the Jito network for block data and tip routing"
)]
struct Args {
    /// Node type (publisher, relayer)
    #[clap(long, Option("publisher", "relayer"))]
    node_type: String,

    /// Node index (0, 1, 2)
    #[clap(short, long)]
    index: u16,
    
    /// Base port number
    #[clap(short, long, default_value = "9000")]
    base_port: u16,
    
    /// Enable Jito tip routing
    #[clap(long)]
    enable_tip_route: bool,

    /// Bootstrap peers to connect to
    #[clap(long, value_delimiter = ',')]
    bootstrap_peers: Vec<String>,

    /// Solana RPC URL
    #[clap(long, default_value = "http://localhost:8899")]
    solana_rpc: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Configure logging with better defaults
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new(format!(
                "windexer_network=info,windexer_jito_staking=info,node_{}=info",
                args.index
            ))
        });

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_ansi(true)
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();

    let port = args.base_port + args.index;
    let rpc_port = port + 1000;
    let metrics_port = 9100 + args.index;
    
    info!("🔧 Configuring node {} with ports:", args.index);
    info!("   P2P: {}", port);
    info!("   RPC: {}", rpc_port);
    info!("   Metrics: {}", metrics_port);

    let config: Box<dyn NodeConfig> = match args.node_type.to_lowercase().as_str() {
        "publisher" => Box::new(PublisherNodeConfig {
            node_id: format!("publisher_{}", args.index),
            node_type: NodeType::PUBLISHER,
            listen_addr: format!("127.0.0.1:{}", port).parse()?,
            rpc_addr: format!("127.0.0.1:{}", rpc_port).parse()?,
            bootstrap_peers: args.bootstrap_peers,
            data_dir: format!("./data/node_{}", args.index),
            solana_rpc_url: args.solana_rpc,
            keypair: SerializableKeypair::new(&Keypair::new()),
            geyser_plugin_config: None,
            metrics_addr: Some(format!("127.0.0.1:{}", metrics_port).parse()?),
        }),
        "relayer" => Box::new(RelayerNodeConfig {
            node_id: format!("relayer_{}", args.index),
            node_type: NodeType::RELAYER,
            listen_addr: format!("127.0.0.1:{}", port).parse()?,
            rpc_addr: format!("127.0.0.1:{}", rpc_port).parse()?,
            bootstrap_peers: args.bootstrap_peers,
            data_dir: format!("./data/node_{}", args.index),
            solana_rpc_url: args.solana_rpc,
            keypair: SerializableKeypair::new(&Keypair::new()),
            geyser_plugin_config: None,
            metrics_addr: Some(format!("127.0.0.1:{}", metrics_port).parse()?),
        }),
        _ => return Err(anyhow!("Invalid node type. Must be 'publisher' or 'relayer'")),
    };

    let staking_config = StakingConfig {
        min_stake: 100_000, // 100k minimum stake
        commission_bps: if args.index == 0 { 0 } else { 500 }, // Bootstrap node has 0% fee
        min_delegation_period: Duration::from_secs(300), // 5 min for demo
        max_operator_stake: 1_000_000_000,
        min_operators: 3,
        consensus_threshold: 0.67,
        reward_rate: 0.15,
        distribution_interval: Duration::from_secs(60), // Every minute for demo
        slash_threshold: 0.90, // 10% downtime allowed
        min_uptime: 0.95,
    };

    info!("🚀 Starting Jito-integrated node {} on port {}", args.index, port);
    
    let (mut node, shutdown_tx) = Node::new(config, staking_config).await?;
    
    // Set up graceful shutdown
    let shutdown_complete = Arc::new(AtomicBool::new(false));
    let shutdown_complete_clone = shutdown_complete.clone();

    ctrlc::set_handler(move || {
        info!("Received shutdown signal");
        if !shutdown_complete_clone.load(Ordering::SeqCst) {
            if let Err(e) = shutdown_tx.try_send(()) {
                warn!("Failed to send shutdown signal: {}", e);
            }
            shutdown_complete_clone.store(true, Ordering::SeqCst);
        }
    })?;

    // Start the node
    node.start().await?;
    
    info!("✅ Node shutdown complete");
    Ok(())
}

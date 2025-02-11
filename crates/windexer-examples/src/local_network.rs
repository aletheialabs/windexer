// examples/local_network.rs

use {
    anyhow::Result,
    solana_sdk::signer::keypair::Keypair,
    std::time::Duration,
    tokio,
    tracing::{info, Level},
    windexer_common::{
        config::NodeConfig,
        crypto::SerializableKeypair,
    },
    windexer_jito_staking::StakingConfig,
    windexer_network::Node,
};

#[tokio::main]
async fn main() -> Result<()> {
    run_local_network(3, 9000).await
}

async fn run_local_network(num_nodes: u16, base_port: u16) -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting local wIndexer network...");

    // Create staking configuration
    let staking_config = StakingConfig {
        min_stake: 1000,
        commission_bps: 500,
        min_delegation_period: Duration::from_secs(86400),
        max_operator_stake: 1_000_000_000_000,
        min_operators: 4,
        consensus_threshold: 0.67,
        reward_rate: 0.10,
        distribution_interval: Duration::from_secs(86400),
        slash_threshold: 0.95,
        min_uptime: 0.99,
    };

    let mut handles = Vec::new();
    
    for i in 0..num_nodes {
        let port = base_port + i;
        let rpc_port = port + 1000;
        
        let config = NodeConfig {
            node_id: format!("node_{}", i),
            listen_addr: format!("127.0.0.1:{}", port).parse()?,
            rpc_addr: format!("127.0.0.1:{}", rpc_port).parse()?,
            bootstrap_peers: vec![],
            data_dir: format!("./data/node_{}", i),
            solana_rpc_url: "http://localhost:8899".to_string(),
            keypair: SerializableKeypair::new(&Keypair::new()),
            geyser_plugin_config: None,
            metrics_addr: None,
        };

        let staking_config = staking_config.clone();

        let handle = tokio::spawn(async move {
            info!("Starting node {} on port {}", i, config.listen_addr);
            
            let (mut node, _shutdown_tx) = Node::new(
                config,
                staking_config,
            ).await?;

            node.start().await
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await??;
    }

    Ok(())
}
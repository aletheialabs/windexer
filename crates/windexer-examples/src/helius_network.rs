use anyhow::{anyhow, Result};
use clap::Parser;
use std::sync::Arc;
use tokio::time::Duration;
use windexer_common::config::NodeConfig;
use windexer_common::crypto::SerializableKeypair;
use windexer_network::Node;
use windexer_network::node::HeliusDataFetcher;
use solana_sdk::signer::keypair::Keypair;
use tracing::{info, error};

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "Example application demonstrating Helius integration with the network",
    long_about = "A sample application showing how to use the Helius API with the wIndexer network"
)]
struct Args {
    /// Helius API key
    #[clap(long, env = "HELIUS_API_KEY")]
    api_key: String,

    /// Network to use (mainnet, devnet, testnet)
    #[clap(long, default_value = "mainnet")]
    network: String,

    /// Port to listen on
    #[clap(long, default_value = "9000")]
    port: u16,

    /// Bootstrap peers to connect to
    #[clap(long, value_delimiter = ',')]
    bootstrap_peers: Vec<String>,

    /// Whether to fetch historical data
    #[clap(long)]
    fetch_historical: bool,
    
    /// Duration to run the example for (in seconds)
    #[clap(long, default_value = "60")]
    duration: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for logs
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();

    // Parse command line arguments
    let args = Args::parse();
    
    // Create node configuration
    let node_config = NodeConfig {
        node_id: "helius-network-example".to_string(),
        listen_addr: format!("127.0.0.1:{}", args.port).parse()?,
        rpc_addr: format!("127.0.0.1:{}", args.port + 1000).parse()?,
        bootstrap_peers: args.bootstrap_peers,
        data_dir: "./data/helius_network_example".to_string(),
        solana_rpc_url: format!("https://{}.helius-rpc.com/?api-key={}", args.network, args.api_key),
        keypair: SerializableKeypair::new(&Keypair::new()),
        geyser_plugin_config: None,
        metrics_addr: Some(format!("127.0.0.1:{}", args.port + 2000).parse()?),
    };
    
    // Create the node
    info!("Creating wIndexer node");
    let (mut node, shutdown_tx) = Node::create_simple(node_config).await?;
    
    // Initialize Helius data fetcher
    info!("Initializing Helius data fetcher");
    node.init_helius_data_fetcher(&args.api_key).await?;
    
    // If we have a data fetcher, use it
    if let Some(data_fetcher) = node.helius_data_fetcher() {
        // Fetch latest slot
        match data_fetcher.get_latest_slot().await {
            Ok(slot) => {
                info!("Latest slot: {}", slot);
                
                // Fetch latest block
                match data_fetcher.get_block(slot).await {
                    Ok(block) => {
                        info!("Latest block: {:?}", block);
                    },
                    Err(e) => {
                        error!("Failed to fetch latest block: {}", e);
                    }
                }
            },
            Err(e) => {
                error!("Failed to fetch latest slot: {}", e);
            }
        }
        
        // If historical data fetching is enabled, fetch known data
        if args.fetch_historical {
            // Fetch some known accounts (example: SOL, USDC)
            let known_accounts = [
                "So11111111111111111111111111111111111111112", // Wrapped SOL
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
            ];
            
            for account in &known_accounts {
                info!("Fetching data for known account: {}", account);
                
                match data_fetcher.get_account(account).await {
                    Ok(account_data) => {
                        info!("Account data for {}: {:?}", account, account_data);
                    },
                    Err(e) => {
                        error!("Failed to fetch account data for {}: {}", account, e);
                    }
                }
            }
            
            // Fetch a known transaction
            let known_tx = "4bBx3YQXmGuLzFxDS5KZNB9A8VfvLQgQJSQ9TUWPiLQPGi2URJ7USRmZA33pfwWnWGRbWy9GtRgaFLZCGYdKyBf1";
            info!("Fetching historical transaction: {}", known_tx);
            
            match data_fetcher.get_transaction(known_tx).await {
                Ok(tx_data) => {
                    info!("Historical transaction: {:?}", tx_data);
                },
                Err(e) => {
                    error!("Failed to fetch historical transaction: {}", e);
                }
            }
            
            // Subscribe to some accounts
            for account in &known_accounts {
                info!("Subscribing to account updates for: {}", account);
                if let Err(e) = data_fetcher.subscribe_account(account).await {
                    error!("Failed to subscribe to account updates: {}", e);
                }
            }
            
            // Subscribe to some programs
            let known_programs = [
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", // Token Program
                "11111111111111111111111111111111", // System Program
            ];
            
            for program in &known_programs {
                info!("Subscribing to program updates for: {}", program);
                if let Err(e) = data_fetcher.subscribe_program(program).await {
                    error!("Failed to subscribe to program updates: {}", e);
                }
            }
        }
    } else {
        error!("Helius data fetcher not initialized");
        return Err(anyhow!("Helius data fetcher not initialized"));
    }
    
    // Start the node
    info!("Starting node...");
    let node_handle = tokio::spawn(async move {
        if let Err(e) = node.start().await {
            error!("Node error: {}", e);
        }
    });
    
    // Run for the specified duration
    info!("Running for {} seconds...", args.duration);
    tokio::time::sleep(Duration::from_secs(args.duration)).await;
    
    // Shutdown
    info!("Shutting down...");
    let _ = shutdown_tx.send(()).await;
    
    // Wait for the node to shut down
    let _ = tokio::time::timeout(Duration::from_secs(5), node_handle).await;
    
    info!("Example completed");
    Ok(())
} 
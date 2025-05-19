use anyhow::Result;
use clap::Parser;
use tokio::time::Duration;
use std::sync::Arc;
use windexer_common::helius::{HeliusClient, HeliusConfig};
use tracing::{info, error};

/// CLI arguments for the Helius example
#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "Example application demonstrating Helius API usage",
    long_about = "A sample application showing how to use the Helius API to fetch blockchain data"
)]
struct Args {
    /// Helius API key
    #[clap(long, env = "HELIUS_API_KEY")]
    api_key: String,

    /// Network to use (mainnet, devnet, testnet)
    #[clap(long, default_value = "mainnet")]
    network: String,

    /// Account to monitor
    #[clap(long)]
    account: Option<String>,

    /// Program ID to monitor
    #[clap(long)]
    program: Option<String>,

    /// Whether to enable WebSocket subscriptions
    #[clap(long)]
    enable_websocket: bool,
    
    /// Whether to fetch historical data
    #[clap(long)]
    fetch_historical: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for logs
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();

    // Parse command line arguments
    let args = Args::parse();
    
    // Create Helius client
    info!("Initializing Helius client for network: {}", args.network);
    let helius_config = HeliusConfig {
        api_key: args.api_key.clone(),
        network: args.network.clone(),
        ws_endpoint: None,
        http_endpoint: None,
    };
    
    let helius_client = Arc::new(HeliusClient::new(helius_config));
    
    // Fetch the latest slot
    match helius_client.get_latest_slot().await {
        Ok(slot) => {
            info!("Latest slot: {}", slot);
            
            // Fetch latest block
            match helius_client.get_block(slot).await {
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
    
    // If an account is specified, fetch its data
    if let Some(account) = &args.account {
        info!("Fetching account data for: {}", account);
        match helius_client.get_account(account).await {
            Ok(account_data) => {
                info!("Account data: {:?}", account_data);
            },
            Err(e) => {
                error!("Failed to fetch account data: {}", e);
            }
        }
    }
    
    // If WebSocket subscriptions are enabled, set them up
    if args.enable_websocket {
        info!("Setting up WebSocket subscriptions");
        
        match helius_client.connect_websocket().await {
            Ok(_) => {
                info!("Connected to WebSocket");
                
                // Subscribe to slot updates
                if let Err(e) = helius_client.subscribe_slots().await {
                    error!("Failed to subscribe to slot updates: {}", e);
                }
                
                // If an account is specified, subscribe to its updates
                if let Some(account) = &args.account {
                    info!("Subscribing to account updates for: {}", account);
                    if let Err(e) = helius_client.subscribe_account(account).await {
                        error!("Failed to subscribe to account updates: {}", e);
                    }
                    
                    info!("Subscribing to transaction updates for: {}", account);
                    if let Err(e) = helius_client.subscribe_signatures(account).await {
                        error!("Failed to subscribe to transaction updates: {}", e);
                    }
                }
                
                // If a program is specified, subscribe to its updates
                if let Some(program) = &args.program {
                    info!("Subscribing to program updates for: {}", program);
                    if let Err(e) = helius_client.subscribe_program(program).await {
                        error!("Failed to subscribe to program updates: {}", e);
                    }
                }
                
                // Set up listeners for updates
                let mut account_rx = helius_client.subscribe_account_updates();
                let mut tx_rx = helius_client.subscribe_transaction_updates();
                let mut block_rx = helius_client.subscribe_block_updates();
                
                // Spawn a task to handle updates
                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            Ok(account) = account_rx.recv() => {
                                info!("Received account update: {}", account.pubkey);
                            },
                            Ok(tx) = tx_rx.recv() => {
                                info!("Received transaction update: {}", tx.signature);
                            },
                            Ok(block) = block_rx.recv() => {
                                info!("Received block update: {}", block.slot);
                            },
                        }
                    }
                });
                
                // Let the example run for a while to receive updates
                info!("Waiting for WebSocket updates (30 seconds)...");
                tokio::time::sleep(Duration::from_secs(30)).await;
            },
            Err(e) => {
                error!("Failed to connect to WebSocket: {}", e);
            }
        }
    }
    
    // If historical data fetching is enabled
    if args.fetch_historical {
        info!("Fetching historical data");
        
        // Fetch a known transaction (example: a recent JUP transaction)
        let known_tx = "4bBx3YQXmGuLzFxDS5KZNB9A8VfvLQgQJSQ9TUWPiLQPGi2URJ7USRmZA33pfwWnWGRbWy9GtRgaFLZCGYdKyBf1";
        info!("Fetching historical transaction: {}", known_tx);
        
        match helius_client.get_transaction(known_tx).await {
            Ok(tx_data) => {
                info!("Historical transaction: {:?}", tx_data);
            },
            Err(e) => {
                error!("Failed to fetch historical transaction: {}", e);
            }
        }
        
        // Fetch some known accounts (example: SOL, USDC)
        let known_accounts = vec![
            "So11111111111111111111111111111111111111112", // Wrapped SOL
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
        ];
        
        for account in known_accounts {
            info!("Fetching data for known account: {}", account);
            
            match helius_client.get_account(account).await {
                Ok(account_data) => {
                    info!("Account data for {}: {:?}", account, account_data);
                },
                Err(e) => {
                    error!("Failed to fetch account data for {}: {}", account, e);
                }
            }
        }
    }
    
    info!("Example completed");
    Ok(())
} 
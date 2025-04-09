use std::net::SocketAddr;
use std::str::FromStr;
use anyhow::Result;
use windexer_api::{ApiServer, ApiConfig, SolanaCluster};

#[tokio::main]
async fn main() -> Result<()> {
    let port = std::env::var("API_PORT")
        .unwrap_or_else(|_| "10001".to_string())
        .parse::<u16>()
        .map_err(|e| anyhow::anyhow!("Invalid API_PORT: {}", e))?;
    
    let log_level = std::env::var("LOG_LEVEL")
        .unwrap_or_else(|_| "info".to_string());
    
    // Get Solana cluster from environment variable
    let solana_cluster = match std::env::var("SOLANA_CLUSTER").unwrap_or_else(|_| "localnet".to_string()).to_lowercase().as_str() {
        "mainnet" | "mainnet-beta" => SolanaCluster::MainnetBeta,
        "testnet" => SolanaCluster::Testnet,
        "devnet" => SolanaCluster::Devnet,
        "localnet" => SolanaCluster::Localnet,
        "custom" => SolanaCluster::Custom,
        _ => SolanaCluster::Localnet,
    };
    
    let config = ApiConfig {
        bind_addr: SocketAddr::from_str(&format!("0.0.0.0:{}", port))?,
        log_level,
        solana_cluster,
    };
    
    let server = ApiServer::new(config);
    server.start().await?;
    
    Ok(())
} 
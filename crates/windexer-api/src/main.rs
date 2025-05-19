use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use anyhow::Result;
use tracing::{info, error};

// Use the local modules instead of windexer_api
use crate::server::run_api_server;
use crate::rest::{ApiServer, ApiConfig};
use crate::types::NodeInfo;

// Local modules
mod account_data_manager;
mod account_endpoints;
mod block_endpoints;
mod endpoints;
mod health;
mod helius;
mod metrics;
mod rest;
mod server;
mod transaction_data_manager;
mod transaction_endpoints;
mod types;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber only once, with proper guard
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .finish();
    
    // Try to set the subscriber as the global default
    if let Err(e) = tracing::subscriber::set_global_default(subscriber) {
        eprintln!("Warning: Failed to set global tracing subscriber: {}", e);
        // Continue anyway, as tracing is non-essential
    }

    // Get configuration from environment
    let port = std::env::var("API_PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .unwrap_or(3001);
    
    let bind_addr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| format!("0.0.0.0:{}", port));
    
    let service_name = std::env::var("SERVICE_NAME")
        .unwrap_or_else(|_| "windexer-api".to_string());
    
    let version = std::env::var("SERVICE_VERSION")
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());

    // Get Helius API key
    let helius_api_key = std::env::var("HELIUS_API_KEY")
        .unwrap_or_else(|_| "test-api-key".to_string());

    // Create node info
    let node_info = Some(NodeInfo {
        node_id: "api-node-1".to_string(),
        node_type: "api".to_string(),
        listen_addr: bind_addr.clone(),
        peer_count: 0,
        is_bootstrap: false,
    });

    // Create API configuration
    let config = ApiConfig {
        bind_addr: SocketAddr::from_str(&bind_addr)?,
        service_name: service_name.clone(),
        version: version.clone(),
        enable_metrics: true,
        node_info: node_info.clone(),
        path_prefix: Some("/api".to_string()),
    };

    // Create and initialize Helius client
    let helius_client = Arc::new(helius::HeliusClient::new(&helius_api_key));

    // Test Helius connection
    match helius_client.get_latest_block().await {
        Ok(_) => info!("Successfully connected to Helius API"),
        Err(e) => {
            error!("Failed to connect to Helius API: {}", e);
            return Err(anyhow::anyhow!("Failed to connect to Helius API: {}", e));
        }
    }

    // Create account data manager
    let account_data_manager = Arc::new(account_data_manager::AccountDataManager::new(helius_client.clone()));

    // Create transaction data manager
    let transaction_data_manager = Arc::new(transaction_data_manager::TransactionDataManager::new(helius_client.clone()));

    // Initialize account data manager
    info!("Initializing account data manager");
    if let Err(e) = account_data_manager.initialize().await {
        tracing::warn!("Failed to initialize account data manager: {}", e);
        // We'll continue even if this fails, as it might be a transient error
    }

    // Initialize transaction data manager
    info!("Initializing transaction data manager");
    if let Err(e) = transaction_data_manager.initialize().await {
        tracing::warn!("Failed to initialize transaction data manager: {}", e);
        // We'll continue even if this fails, as it might be a transient error
    }

    // Create API server with data managers
    let mut server = ApiServer::new(config);
    
    // Set the account data manager
    server.set_account_data_manager(account_data_manager);
    
    // Set the transaction data manager
    server.set_transaction_data_manager(transaction_data_manager);
    
    // Set the Helius client
    server.set_helius_client(helius_client);

    // Register health checks
    let health = server.health();
    health.register("api", Arc::new(|| true)).await;
    
    // Register metrics collection
    let metrics = server.metrics();
    metrics.register_collector(|| {
        let mut metrics = std::collections::HashMap::new();
        metrics.insert("memory_usage".to_string(), serde_json::json!(100));
        metrics.insert("cpu_usage".to_string(), serde_json::json!(5));
        metrics.insert("active_connections".to_string(), serde_json::json!(10));
        metrics
    });

    // Start the server
    info!("Starting API server on {}", bind_addr);
    server.start().await?;

    Ok(())
}
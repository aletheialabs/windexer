use axum::{
    routing::get,
    Router,
    Json,
    extract::Path,
    http::{StatusCode, Method},
    response::{IntoResponse, Response},
};
use serde::{Serialize, Deserialize};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tower_http::cors::{CorsLayer, Any};
use std::sync::Arc;
use anyhow::Result;
use tracing::{info, error};

use crate::server::run_api_server;
use crate::rest::{ApiServer, ApiConfig};
use crate::types::NodeInfo;

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

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StatusResponse {
    name: String,
    version: String,
    uptime: u64,
    timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    uptime: u64,
    timestamp: String,
    checks: std::collections::HashMap<String, bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BlockResponse {
    slot: u64,
    hash: String,
    parent_hash: String,
    block_time: u64,
    block_height: u64,
    transactions: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct TransactionResponse {
    signature: String,
    slot: u64,
    block_time: u64,
    fee: u64,
    status: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .finish();
    
    if let Err(e) = tracing::subscriber::set_global_default(subscriber) {
        eprintln!("Warning: Failed to set global tracing subscriber: {}", e);
    }

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

    let helius_api_key = std::env::var("HELIUS_API_KEY")
        .unwrap_or_else(|_| "test-api-key".to_string());

    let node_info = Some(NodeInfo {
        node_id: "api-node-1".to_string(),
        node_type: "api".to_string(),
        listen_addr: bind_addr.clone(),
        peer_count: 0,
        is_bootstrap: false,
    });

    let config = ApiConfig {
        bind_addr: SocketAddr::from_str(&bind_addr)?,
        service_name: service_name.clone(),
        version: version.clone(),
        enable_metrics: true,
        node_info: node_info.clone(),
        path_prefix: Some("/api".to_string()),
    };

    let helius_client = Arc::new(helius::HeliusClient::new(&helius_api_key));

    match helius_client.get_latest_block().await {
        Ok(_) => info!("Successfully connected to Helius API"),
        Err(e) => {
            error!("Failed to connect to Helius API: {}", e);
            return Err(anyhow::anyhow!("Failed to connect to Helius API: {}", e));
        }
    }

    let account_data_manager = Arc::new(account_data_manager::AccountDataManager::new(helius_client.clone()));

    let transaction_data_manager = Arc::new(transaction_data_manager::TransactionDataManager::new(helius_client.clone()));

    // Initializ account data manager
    info!("Initializing account data manager");
    if let Err(e) = account_data_manager.initialize().await {
        tracing::warn!("Failed to initialize account data manager: {}", e);
        // We'll continue even if this fails, as it might be a transient error
    }

    info!("Initializing transaction data manager");
    if let Err(e) = transaction_data_manager.initialize().await {
        tracing::warn!("Failed to initialize transaction data manager: {}", e);
        // We'll continue even if this fails, as it might be a transient error
    }

    let mut server = ApiServer::new(config);
    
    server.set_account_data_manager(account_data_manager);
    server.set_transaction_data_manager(transaction_data_manager);
    server.set_helius_client(helius_client);
    let health = server.health();
    health.register("api", Arc::new(|| true)).await;
    
    let metrics = server.metrics();
    metrics.register_collector(|| {
        let mut metrics = std::collections::HashMap::new();
        metrics.insert("memory_usage".to_string(), serde_json::json!(100));
        metrics.insert("cpu_usage".to_string(), serde_json::json!(5));
        metrics.insert("active_connections".to_string(), serde_json::json!(10));
        metrics
    });

    info!("Starting API server on {}", bind_addr);
    server.start().await?;

    Ok(())
}

async fn status_handler() -> Json<ApiResponse<StatusResponse>> {
    let start_time = SystemTime::now().checked_sub(Duration::from_secs(3600)).unwrap_or(UNIX_EPOCH);
    
    let status = StatusResponse {
        name: "windexer-api".to_string(),
        version: "0.1.0".to_string(),
        uptime: SystemTime::now().duration_since(start_time).unwrap_or(Duration::from_secs(0)).as_secs(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    
    Json(ApiResponse::success(status))
}

async fn health_handler() -> Json<HealthResponse> {
    let start_time = SystemTime::now().checked_sub(Duration::from_secs(3600)).unwrap_or(UNIX_EPOCH);
    
    let mut checks = std::collections::HashMap::new();
    checks.insert("api".to_string(), true);
    
    let response = HealthResponse {
        status: "healthy".to_string(),
        uptime: SystemTime::now().duration_since(start_time).unwrap_or(Duration::from_secs(0)).as_secs(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        checks,
    };
    
    Json(response)
}

async fn latest_block_handler() -> Json<ApiResponse<BlockResponse>> {
    let block = BlockResponse {
        slot: 123456789,
        hash: "3SnrsLVuVoupUhBAnYDJ9zxygyHJ5sY9i3FZwmgBVWqB".to_string(),
        parent_hash: "H4vxhecSR1JUdr5n8MLFSgEDLJkdCtQQgXgUL3EG85KC".to_string(),
        block_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs(),
        block_height: 123456700,
        transactions: 100,
    };
    
    Json(ApiResponse::success(block))
}

async fn block_by_slot_handler(Path(slot): Path<String>) -> Response {
    let slot_number = match slot.parse::<u64>() {
        Ok(num) => num,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!("Invalid slot number: {}", slot))),
            ).into_response();
        }
    };
    
    let block = BlockResponse {
        slot: slot_number,
        hash: "3SnrsLVuVoupUhBAnYDJ9zxygyHJ5sY9i3FZwmgBVWqB".to_string(),
        parent_hash: "H4vxhecSR1JUdr5n8MLFSgEDLJkdCtQQgXgUL3EG85KC".to_string(),
        block_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs(),
        block_height: slot_number.saturating_sub(100),
        transactions: 100,
    };
    
    Json(ApiResponse::success(block)).into_response()
}

async fn transaction_handler(Path(signature): Path<String>) -> Json<ApiResponse<TransactionResponse>> {
    let tx = TransactionResponse {
        signature: signature,
        slot: 123456789,
        block_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs(),
        fee: 5000,
        status: "confirmed".to_string(),
    };
    
    Json(ApiResponse::success(tx))
} 
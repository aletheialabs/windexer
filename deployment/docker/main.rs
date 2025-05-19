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
async fn main() -> anyhow::Result<()> {
    // Setup logging
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set up global logger");

    // Start time
    let start_time = SystemTime::now();
    
    // Get port from environment
    let port = std::env::var("API_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap_or(3000);
    
    // Bind address
    let bind_addr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| format!("0.0.0.0:{}", port));
    let socket_addr = SocketAddr::from_str(&bind_addr)?;

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_origin(Any);

    // Build router
    let app = Router::new()
        // Health endpoints
        .route("/api/health", get(health_handler))
        .route("/api/status", get(status_handler))
        
        // Block endpoints
        .route("/api/blocks/latest", get(latest_block_handler))
        .route("/api/blocks/:slot", get(block_by_slot_handler))
        
        // Transaction endpoints
        .route("/api/transaction/:signature", get(transaction_handler))
        
        // Apply CORS
        .layer(cors);

    // Start the server
    let listener = tokio::net::TcpListener::bind(socket_addr).await?;
    tracing::info!("Listening on http://{}", socket_addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn status_handler() -> Json<ApiResponse<StatusResponse>> {
    // Start time of the server (for this example we use a fixed time)
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
    // Start time of the server (for this example we use a fixed time)
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
    // Static block data for demonstration
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
    // Try to parse the slot
    let slot_number = match slot.parse::<u64>() {
        Ok(num) => num,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!("Invalid slot number: {}", slot))),
            ).into_response();
        }
    };
    
    // Static block data for demonstration
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
    // Static transaction data for demonstration
    let tx = TransactionResponse {
        signature: signature,
        slot: 123456789,
        block_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs(),
        fee: 5000,
        status: "confirmed".to_string(),
    };
    
    Json(ApiResponse::success(tx))
} 
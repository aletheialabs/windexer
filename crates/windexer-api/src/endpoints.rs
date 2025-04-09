// crates/windexer-api/src/endpoints.rs

use axum::{
    routing::{get, post},
    Json, extract::{State, Path, Query},
    response::{IntoResponse, Response},
    http::StatusCode,
    Router,
};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::types::{ApiResponse, ApiError};
use crate::rest::AppState;

// Add more query parameters for endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionQuery {
    pub limit: Option<usize>,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountQuery {
    pub limit: Option<usize>,
    pub address: Option<String>,
    pub owner: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidatorInfo {
    pub identity: String,
    pub version: String,
    pub cluster: String,
    pub features: Vec<String>,
    pub metrics: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeploymentInfo {
    pub id: String,
    pub environment: String,
    pub timestamp: String,
    pub version: String,
    pub config: HashMap<String, serde_json::Value>,
    pub nodes: Vec<NodeSummary>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeSummary {
    pub id: String,
    pub node_type: String,
    pub status: String,
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeploymentConfig {
    pub node_count: usize,
    pub indexer_count: usize,
    pub base_port: u16,
    pub validator_config: HashMap<String, serde_json::Value>,
    pub env_vars: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateDeploymentRequest {
    pub config: DeploymentConfig,
    pub restart: bool,
}

// Transaction data structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransactionData {
    pub signature: String,
    pub block_time: Option<i64>,
    pub slot: u64,
    pub success: bool,
    pub fee: u64,
    pub accounts: Vec<String>,
}

// Account data structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountData {
    pub pubkey: String,
    pub lamports: u64,
    pub owner: String,
    pub executable: bool,
    pub rent_epoch: u64,
    pub data_len: usize,
    pub write_version: u64,
}

pub async fn get_deployment_info(
    State(state): State<AppState>
) -> Json<ApiResponse<DeploymentInfo>> {
    // Get cluster information from Solana client
    let validator_info = state.solana_client.get_validator_info().await
        .unwrap_or_else(|_| serde_json::json!({}));
    
    let cluster = validator_info.get("cluster")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    
    // Build deployment info with real validator information
    let info = DeploymentInfo {
        id: "windexer-real-data".to_string(),
        environment: cluster.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        config: HashMap::new(),
        nodes: vec![
            NodeSummary {
                id: "api-server".to_string(),
                node_type: "api".to_string(),
                status: "running".to_string(),
                address: validator_info.get("rpc_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
            },
        ],
    };
    
    Json(ApiResponse::success(info))
}

pub async fn update_deployment(
    State(state): State<AppState>,
    Json(request): Json<UpdateDeploymentRequest>
) -> Json<ApiResponse<DeploymentInfo>> {
    // Convert to HashMap manually
    let mut config_map = HashMap::new();
    if let Ok(value) = serde_json::to_value(&request.config) {
        if let Some(obj) = value.as_object() {
            for (k, v) in obj {
                config_map.insert(k.clone(), v.clone());
            }
        }
    }
    
    // Get cluster information from Solana client
    let validator_info = state.solana_client.get_validator_info().await
        .unwrap_or_else(|_| serde_json::json!({}));
    
    let cluster = validator_info.get("cluster")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    
    // Build deployment info with real validator information
    let info = DeploymentInfo {
        id: "windexer-real-data".to_string(),
        environment: cluster.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        config: config_map,
        nodes: vec![
            NodeSummary {
                id: "api-server".to_string(),
                node_type: "api".to_string(),
                status: "updating".to_string(),
                address: validator_info.get("rpc_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
            },
        ],
    };
    
    Json(ApiResponse::success(info))
}

pub async fn get_validator_info(
    State(state): State<AppState>
) -> Json<ApiResponse<serde_json::Value>> {
    // Use real validator info from Solana client
    let result = match state.solana_client.get_validator_info().await {
        Ok(info) => info,
        Err(e) => {
            return Json(ApiResponse::error(format!("Failed to get validator info: {}", e)));
        }
    };
    
    Json(ApiResponse::success(result))
}

// Get recent transactions from Solana
pub async fn get_transactions(
    State(state): State<AppState>,
    Query(params): Query<TransactionQuery>,
) -> Json<ApiResponse<Vec<TransactionData>>> {
    // Get the limit parameter, defaulting to 10
    let limit = params.limit.unwrap_or(10);
    
    // Use real transactions from Solana client
    match state.solana_client.get_recent_transactions(limit).await {
        Ok(transactions) => Json(ApiResponse::success(transactions)),
        Err(e) => Json(ApiResponse::error(format!("Failed to get transactions: {}", e))),
    }
}

// Get accounts from Solana
pub async fn get_accounts(
    State(state): State<AppState>,
    Query(params): Query<AccountQuery>,
) -> Json<ApiResponse<Vec<AccountData>>> {
    // Get the limit parameter, defaulting to 10
    let limit = params.limit.unwrap_or(10);
    
    // Build address list from query parameters
    let mut addresses = Vec::new();
    if let Some(address) = params.address {
        addresses.push(address);
    }
    
    // Use real accounts from Solana client
    match state.solana_client.get_accounts(&addresses, limit).await {
        Ok(accounts) => Json(ApiResponse::success(accounts)),
        Err(e) => Json(ApiResponse::error(format!("Failed to get accounts: {}", e))),
    }
}

pub fn create_deployment_router() -> Router<AppState> {
    Router::new()
        .route("/deployment", get(get_deployment_info))
        .route("/deployment", post(update_deployment))
        .route("/validator", get(get_validator_info))
        .route("/transactions", get(get_transactions))
        .route("/accounts", get(get_accounts))
} 
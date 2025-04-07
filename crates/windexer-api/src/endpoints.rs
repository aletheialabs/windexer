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

pub async fn get_deployment_info(
    State(state): State<AppState>
) -> Json<ApiResponse<DeploymentInfo>> {
    // Placeholder for actual implementation
    let info = DeploymentInfo {
        id: "windexer-default".to_string(),
        environment: "development".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        config: HashMap::new(),
        nodes: vec![
            NodeSummary {
                id: "node-0".to_string(),
                node_type: "core".to_string(),
                status: "running".to_string(),
                address: "localhost:9000".to_string(),
            },
            NodeSummary {
                id: "indexer-0".to_string(),
                node_type: "indexer".to_string(),
                status: "running".to_string(),
                address: "localhost:10000".to_string(),
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
    
    // Placeholder for actual implementation
    let info = DeploymentInfo {
        id: "windexer-default".to_string(),
        environment: "development".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        config: config_map,
        nodes: vec![
            NodeSummary {
                id: "node-0".to_string(),
                node_type: "core".to_string(),
                status: "restarting".to_string(),
                address: "localhost:9000".to_string(),
            },
            NodeSummary {
                id: "indexer-0".to_string(),
                node_type: "indexer".to_string(),
                status: "restarting".to_string(),
                address: "localhost:10000".to_string(),
            },
        ],
    };
    
    Json(ApiResponse::success(info))
}

pub async fn get_validator_info(
    State(state): State<AppState>
) -> Json<ApiResponse<ValidatorInfo>> {
    // Placeholder for actual implementation
    let info = ValidatorInfo {
        identity: "windexer-validator".to_string(),
        version: "1.16.0".to_string(),
        cluster: "localnet".to_string(),
        features: vec![
            "geyser-plugin".to_string(),
            "account-indexing".to_string(),
            "transaction-indexing".to_string(),
        ],
        metrics: HashMap::new(),
    };
    
    Json(ApiResponse::success(info))
}

pub fn create_deployment_router() -> Router<AppState> {
    Router::new()
        .route("/deployment", get(get_deployment_info))
        .route("/deployment", post(update_deployment))
        .route("/validator", get(get_validator_info))
} 
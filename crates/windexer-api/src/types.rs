use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use axum::{response::IntoResponse, http::StatusCode, Json};

/// Standard API response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Success status
    pub success: bool,
    /// Optional result data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Optional error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    /// Create a successful response with data
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    /// Create an error response
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// API error types
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    
    #[error("Forbidden: {0}")]
    Forbidden(String),
}

/// Convert ApiError to HTTP response
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
        };
        
        let body = Json(ApiResponse::<()>::error(error_message));
        
        (status, body).into_response()
    }
}

/// Status response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    /// Service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Service uptime in seconds
    pub uptime: u64,
    /// Current time in ISO 8601 format
    pub timestamp: String,
    /// Additional status fields
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub additional: HashMap<String, serde_json::Value>,
}

/// Health check response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall health status
    pub status: HealthStatus,
    /// Detailed checks
    pub checks: HashMap<String, HealthCheckResult>,
    /// Service uptime in seconds
    pub uptime: u64,
}

/// Health status enum
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All systems operational
    Healthy,
    /// Some systems degraded
    Degraded,
    /// Critical systems failing
    Unhealthy,
}

/// Individual health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Check status
    pub status: HealthStatus,
    /// Details about the check
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// Optional metrics related to this check
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<HashMap<String, serde_json::Value>>,
}

/// Node information for status responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node ID
    pub node_id: String,
    /// Node type (e.g., "core", "indexer", etc.)
    pub node_type: String,
    /// Listen address
    pub listen_addr: String,
    /// Connected peers count
    pub peer_count: usize,
    /// Whether this node is a bootstrap node
    pub is_bootstrap: bool,
}
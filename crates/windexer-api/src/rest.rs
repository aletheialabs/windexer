// crates/windexer-api/src/rest.rs

use axum::{
    Router,
    routing::get,
    extract::State,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::health::HealthService;
use crate::metrics::MetricsService;
use crate::endpoints::create_deployment_router;
use crate::types::HealthResponse;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Health check service
    pub health: Arc<HealthService>,
    /// Metrics service
    pub metrics: Arc<MetricsService>,
    /// Application configuration
    pub config: Arc<RwLock<serde_json::Value>>,
}

/// Create the API router with all routes
pub fn create_api_router() -> Router {
    // Create services
    let health_service = Arc::new(HealthService::new());
    let metrics_service = Arc::new(MetricsService::new());
    
    // Create application state
    let state = AppState {
        health: health_service.clone(),
        metrics: metrics_service.clone(),
        config: Arc::new(RwLock::new(serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "name": env!("CARGO_PKG_NAME"),
        }))),
    };
    
    // Create router
    Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/status", get(status_handler))
        .route("/api/metrics", get(metrics_handler))
        .nest("/api", create_deployment_router())
        .with_state(state)
}

/// Health check handler
async fn health_handler(
    State(state): State<AppState>
) -> axum::Json<HealthResponse> {
    let response = state.health.check_all().await;
    axum::Json(response)
}

/// Status handler
async fn status_handler(
    State(state): State<AppState>
) -> axum::Json<serde_json::Value> {
    let config = state.config.read().await;
    axum::Json(config.clone())
}

/// Metrics handler
async fn metrics_handler(
    State(state): State<AppState>
) -> axum::Json<serde_json::Value> {
    let metrics = state.metrics.get_metrics().await;
    axum::Json(metrics)
} 
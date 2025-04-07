// src/lib.rs

//! windexer-api - API server for wIndexer deployment
//! 
//! Provides a unified HTTP API for monitoring and managing wIndexer deployments.

use std::net::SocketAddr;

// Public modules
pub mod types;
pub mod health;
pub mod metrics;
pub mod endpoints;
pub mod rest;

// Re-exports 
pub use types::{ApiResponse, ApiError, StatusResponse, HealthResponse, HealthStatus, HealthCheckResult, NodeInfo};
pub use health::HealthService;
pub use metrics::MetricsService;
pub use rest::AppState;

/// API server configuration
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Server bind address
    pub bind_addr: SocketAddr,
    /// Log level (default: info)
    pub log_level: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:3000".parse().expect("Valid default bind address"),
            log_level: "info".to_string(),
        }
    }
}

/// API server
pub struct ApiServer {
    /// Server configuration
    config: ApiConfig,
}

impl ApiServer {
    /// Create a new API server with the given configuration
    pub fn new(config: ApiConfig) -> Self {
        Self { config }
    }
    
    /// Start the API server
    pub async fn start(&self) -> anyhow::Result<()> {
        // Initialize tracing
        tracing_subscriber::fmt()
            .with_env_filter(format!("windexer_api={}", self.config.log_level))
            .init();

        tracing::info!("Starting wIndexer API server on {}", self.config.bind_addr);
        
        // Create the router
        let router = rest::create_api_router();
        
        // Start the server
        let listener = tokio::net::TcpListener::bind(self.config.bind_addr).await?;
        tracing::info!("Listening on {}", self.config.bind_addr);
        
        axum::serve(listener, router).await?;
        
        Ok(())
    }
}

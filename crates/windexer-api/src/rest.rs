use axum::{
    Router,
    routing::get,
    extract::State,
    http::{Method, HeaderValue, header},
};
use std::sync::Arc;
use std::time::Instant;
use tower_http::cors::{CorsLayer, Any};
use tokio::sync::RwLock;

use crate::health::HealthService;
use crate::metrics::MetricsService;
use crate::types::{ApiResponse, HealthResponse, StatusResponse};

// Import endpoint modules
use crate::account_endpoints::create_account_router;
use crate::transaction_endpoints::create_transaction_router;
use crate::block_endpoints::create_block_router;
use crate::endpoints::create_deployment_router;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Health check service
    pub health: Arc<HealthService>,
    /// Metrics service
    pub metrics: Arc<MetricsService>,
    /// Application start time
    pub start_time: Instant,
    /// Application configuration
    pub config: Arc<RwLock<serde_json::Value>>,
    /// Service name
    pub service_name: String,
    /// Service version
    pub version: String,
    /// Node information
    pub node_info: Option<crate::types::NodeInfo>,
    /// Account data manager
    pub account_data_manager: Option<Arc<crate::account_data_manager::AccountDataManager>>,
    /// Transaction data manager
    pub transaction_data_manager: Option<Arc<crate::transaction_data_manager::TransactionDataManager>>,
    /// Helius client
    pub helius_client: Option<Arc<crate::helius::HeliusClient>>,
}

/// API server configuration
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Server bind address
    pub bind_addr: std::net::SocketAddr,
    /// Service name
    pub service_name: String,
    /// Service version
    pub version: String,
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Node information (optional)
    pub node_info: Option<crate::types::NodeInfo>,
    /// API path prefix (optional)
    pub path_prefix: Option<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:3000".parse().expect("Valid default bind address"),
            service_name: "windexer-api".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            enable_metrics: true,
            node_info: None,
            path_prefix: Some("/api".to_string()),
        }
    }
}

/// API server
pub struct ApiServer {
    /// Server configuration
    config: ApiConfig,
    /// Health service
    health_service: Arc<HealthService>,
    /// Metrics service
    metrics_service: Arc<MetricsService>,
    /// Application state
    state: AppState,
}

impl ApiServer {
    /// Create a new API server with the given configuration
    pub fn new(config: ApiConfig) -> Self {
        let health_service = Arc::new(HealthService::new());
        let metrics_service = Arc::new(MetricsService::new());

        let state = AppState {
            health: health_service.clone(),
            metrics: metrics_service.clone(),
            start_time: Instant::now(),
            config: Arc::new(RwLock::new(serde_json::json!({
                "service_name": config.service_name,
                "version": config.version,
            }))),
            service_name: config.service_name.clone(),
            version: config.version.clone(),
            node_info: config.node_info.clone(),
            account_data_manager: None,
            transaction_data_manager: None,
            helius_client: None,
        };

        Self {
            config,
            health_service,
            metrics_service,
            state,
        }
    }

    /// Set the account data manager
    pub fn set_account_data_manager(&mut self, account_data_manager: Arc<crate::account_data_manager::AccountDataManager>) {
        self.state.account_data_manager = Some(account_data_manager);
    }

    /// Set the transaction data manager
    pub fn set_transaction_data_manager(&mut self, transaction_data_manager: Arc<crate::transaction_data_manager::TransactionDataManager>) {
        self.state.transaction_data_manager = Some(transaction_data_manager);
    }

    /// Set the Helius client
    pub fn set_helius_client(&mut self, helius_client: Arc<crate::helius::HeliusClient>) {
        self.state.helius_client = Some(helius_client);
    }

    /// Get a reference to the health service
    pub fn health(&self) -> Arc<HealthService> {
        self.health_service.clone()
    }

    /// Get a reference to the metrics service
    pub fn metrics(&self) -> Arc<MetricsService> {
        self.metrics_service.clone()
    }

    /// Start the API server
    pub async fn start(&self) -> anyhow::Result<()> {
        // Log startup
        tracing::info!("Starting {} API server on {}", self.config.service_name, self.config.bind_addr);

        // Create the router
        let router = self.create_router();

        // Start the server
        let listener = tokio::net::TcpListener::bind(self.config.bind_addr).await?;
        tracing::info!("Listening on {}", self.config.bind_addr);

        axum::serve(listener, router).await?;

        Ok(())
    }

    /// Create the API router
    fn create_router(&self) -> Router {
        // Configure CORS
        let cors = CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE])
            .allow_origin(Any);

        // Create base router with core endpoints
        let mut router = Router::new()
            .route("/health", get(health_handler))
            .route("/status", get(status_handler))
            .layer(cors);

        // Add metrics endpoint if enabled
        if self.config.enable_metrics {
            router = router.route("/metrics", get(metrics_handler));
        }

        // Add account, transaction, block and deployment endpoints
        router = router
            .merge(create_account_router())
            .merge(create_transaction_router())
            .merge(create_block_router())
            .merge(create_deployment_router());

        // Apply path prefix if configured
        if let Some(prefix) = &self.config.path_prefix {
            router = Router::new().nest(prefix, router);
        }

        // Add state at the end to ensure correct type
        router.with_state(self.state.clone())
    }
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
) -> axum::Json<ApiResponse<StatusResponse>> {
    let status = StatusResponse {
        name: state.service_name.clone(),
        version: state.version.clone(),
        uptime: state.start_time.elapsed().as_secs(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        additional: std::collections::HashMap::new(),
    };

    axum::Json(ApiResponse::success(status))
}

/// Metrics handler
async fn metrics_handler(
    State(state): State<AppState>
) -> axum::Json<serde_json::Value> {
    let metrics = state.metrics.get_metrics().await;
    axum::Json(metrics)
}
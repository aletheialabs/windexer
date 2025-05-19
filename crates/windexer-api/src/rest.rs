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
use std::net::SocketAddr;
use std::collections::HashMap;
use serde_json::Value;
use tokio::net::TcpListener;
use tracing::{debug, info, error, warn};
use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use axum::extract::ws::WebSocket;
use axum::response::IntoResponse;
use axum::routing::MethodRouter;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;

use crate::health::HealthService;
use crate::metrics::MetricsService;
use crate::types::{ApiResponse, HealthResponse, StatusResponse};

use crate::account_endpoints::create_account_router;
use crate::transaction_endpoints::create_transaction_router;
use crate::block_endpoints::create_block_router;
use crate::endpoints::create_deployment_router;

#[derive(Clone)]
pub struct AppState {
    pub health: Arc<HealthService>,
    pub metrics: Arc<MetricsService>,
    pub start_time: Instant,
    pub config: Arc<RwLock<serde_json::Value>>,
    pub service_name: String,
    pub version: String,
    pub node_info: Option<crate::types::NodeInfo>,
    pub account_data_manager: Option<Arc<crate::account_data_manager::AccountDataManager>>,
    pub transaction_data_manager: Option<Arc<crate::transaction_data_manager::TransactionDataManager>>,
    pub helius_client: Option<Arc<crate::helius::HeliusClient>>,
}

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub bind_addr: std::net::SocketAddr,
    pub service_name: String,
    pub version: String,
    pub enable_metrics: bool,
    pub node_info: Option<crate::types::NodeInfo>,
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

pub struct ApiServer {
    config: ApiConfig,
    health_service: Arc<HealthService>,
    metrics_service: Arc<MetricsService>,
    state: AppState,
}

impl ApiServer {
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

    pub fn set_account_data_manager(&mut self, account_data_manager: Arc<crate::account_data_manager::AccountDataManager>) {
        self.state.account_data_manager = Some(account_data_manager);
    }

    pub fn set_transaction_data_manager(&mut self, transaction_data_manager: Arc<crate::transaction_data_manager::TransactionDataManager>) {
        self.state.transaction_data_manager = Some(transaction_data_manager);
    }

    pub fn set_helius_client(&mut self, helius_client: Arc<crate::helius::HeliusClient>) {
        self.state.helius_client = Some(helius_client);
    }

    pub fn health(&self) -> Arc<HealthService> {
        self.health_service.clone()
    }

    pub fn metrics(&self) -> Arc<MetricsService> {
        self.metrics_service.clone()
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("Starting {} API server on {}", self.config.service_name, self.config.bind_addr);

        let mut router = self.create_router();

        let jito_blocks_router = crate::block_endpoints::create_jito_compat_blocks_router()
            .with_state(self.state.clone());
        let jito_tx_router = crate::transaction_endpoints::create_jito_compat_transaction_router()
            .with_state(self.state.clone());
        
        router = router
            .merge(jito_blocks_router)
            .merge(jito_tx_router);
        
        let listener = tokio::net::TcpListener::bind(self.config.bind_addr).await?;
        tracing::info!("Listening on {}", self.config.bind_addr);

        axum::serve(listener, router).await?;

        Ok(())
    }

    fn create_router(&self) -> Router {
        let cors = CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
            .allow_headers(Any)
            .allow_origin(Any)
            .expose_headers(Any);

        let mut router = Router::new()
            .route("/health", get(health_handler))
            .route("/status", get(status_handler));

        if self.config.enable_metrics {
            router = router.route("/metrics", get(metrics_handler));
        }

        router = router
            .merge(create_account_router())
            .merge(create_transaction_router())
            .merge(create_block_router())
            .merge(create_deployment_router());

        if let Some(prefix) = &self.config.path_prefix {
            router = Router::new().nest(prefix, router);
        }

        router = router.layer(cors);

        router.with_state(self.state.clone())
    }
}

async fn health_handler(
    State(state): State<AppState>
) -> axum::Json<HealthResponse> {
    let response = state.health.check_all().await;
    axum::Json(response)
}

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

async fn metrics_handler(
    State(state): State<AppState>
) -> axum::Json<serde_json::Value> {
    let metrics = state.metrics.get_metrics().await;
    axum::Json(metrics)
}
// src/server.rs

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::RwLock;
use anyhow::Result;
use tracing::{info, error};
use axum::Router;

use crate::rest::{ApiServer, ApiConfig, AppState};
use crate::types::NodeInfo;
use crate::types::{HealthStatus, HealthCheckResult};

pub async fn run_api_server(
    bind_addr: SocketAddr,
    service_name: impl Into<String>,
    version: impl Into<String>,
    node_info: Option<NodeInfo>,
) -> Result<()> {
    let config = ApiConfig {
        bind_addr,
        service_name: service_name.into(),
        version: version.into(),
        enable_metrics: true,
        node_info,
        path_prefix: Some("/api".to_string()),
    };
    
    info!("Starting API server for {} v{}", config.service_name, config.version);
    
    let server = ApiServer::new(config);
    
    let health = server.health();
    
    health.register("system", Arc::new(|| true)).await;
    
    server.start().await?;
    
    Ok(())
}

pub async fn run_api_server_with_config(
    config: ApiConfig,
    shutdown_signal: Option<tokio::sync::oneshot::Receiver<()>>,
) -> Result<()> {
    info!("Starting API server for {} v{}", config.service_name, config.version);
    
    let server = ApiServer::new(config);
    
    // TODO: Implement shutdown handling with the signal
    server.start().await?;
    
    Ok(())
}

pub fn create_url_health_check(
    url: String,
    timeout_ms: u64,
    name: &str,
) -> impl Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = HealthCheckResult> + Send>> {
    use std::time::Duration;
    let name = name.to_string();
    
    move || {
        let url = url.clone();
        let name = name.clone();
        let timeout = Duration::from_millis(timeout_ms);
        
        Box::pin(async move {
            use std::collections::HashMap;
            
            tracing::debug!("Checking health of {} at {}", name, url);
            
            let client = reqwest::Client::new();
            let timer = std::time::Instant::now();
            
            match tokio::time::timeout(timeout, client.get(&url).send()).await {
                Ok(response_result) => {
                    let elapsed = timer.elapsed().as_millis() as u64;
                    
                    match response_result {
                        Ok(response) => {
                            if response.status().is_success() {
                                HealthCheckResult {
                                    status: HealthStatus::Healthy,
                                    details: Some(format!("{} is healthy", name)),
                                    metrics: Some(HashMap::from([
                                        ("response_time_ms".to_string(), serde_json::Value::Number(serde_json::Number::from(elapsed)))
                                    ])),
                                }
                            } else {
                                HealthCheckResult {
                                    status: HealthStatus::Degraded,
                                    details: Some(format!("{} returned error status: {}", name, response.status())),
                                    metrics: Some(HashMap::from([
                                        ("response_time_ms".to_string(), serde_json::Value::Number(serde_json::Number::from(elapsed)))
                                    ])),
                                }
                            }
                        },
                        Err(e) => {
                            HealthCheckResult {
                                status: HealthStatus::Unhealthy,
                                details: Some(format!("Failed to connect to {}: {}", name, e)),
                                metrics: None,
                            }
                        }
                    }
                },
                Err(_) => {
                    HealthCheckResult {
                        status: HealthStatus::Unhealthy,
                        details: Some(format!("{} health check timed out after {}ms", name, timeout_ms)),
                        metrics: None,
                    }
                }
            }
        })
    }
} 
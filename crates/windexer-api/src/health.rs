// src/health.rs

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::types::{HealthStatus, HealthResponse, HealthCheckResult};

// Change from Fn() -> bool to async health checks
pub type HealthCheckFn = Arc<dyn Fn() -> bool + Send + Sync>;
pub type AsyncHealthCheckFn = Arc<dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = HealthCheckResult> + Send>> + Send + Sync>;

pub struct HealthService {
    checks: Arc<RwLock<HashMap<String, HealthCheckFn>>>,
    async_checks: Arc<RwLock<HashMap<String, AsyncHealthCheckFn>>>,
    start_time: Instant,
}

impl HealthService {
    pub fn new() -> Self {
        Self {
            checks: Arc::new(RwLock::new(HashMap::new())),
            async_checks: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    pub async fn register(&self, name: &str, check: HealthCheckFn) {
        let mut checks = self.checks.write().await;
        checks.insert(name.to_string(), check);
    }
    
    // Add a method for registering async health checks
    pub async fn register_async(&self, name: &str, check: AsyncHealthCheckFn) {
        let mut checks = self.async_checks.write().await;
        checks.insert(name.to_string(), check);
    }

    pub async fn unregister(&self, name: &str) {
        let mut checks = self.checks.write().await;
        checks.remove(name);
        
        let mut async_checks = self.async_checks.write().await;
        async_checks.remove(name);
    }

    pub fn uptime(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    pub async fn check_all(&self) -> HealthResponse {
        // First collect all check names from both sync and async checks
        let check_names: Vec<String> = {
            let checks = self.checks.read().await;
            checks.keys().cloned().collect()
        };
        
        let async_check_names: Vec<String> = {
            let checks = self.async_checks.read().await;
            checks.keys().cloned().collect()
        };
        
        let mut results = HashMap::new();
        let mut all_healthy = true;
        let mut any_healthy = false;
        
        // Run synchronous checks
        for name in check_names {
            let check_fn = {
                let checks = self.checks.read().await;
                checks.get(&name).cloned()
            };
            
            if let Some(check) = check_fn {
                let is_healthy = check();
                
                let result = if is_healthy {
                    all_healthy &= true;
                    any_healthy |= true;
                    
                    HealthCheckResult {
                        status: HealthStatus::Healthy,
                        details: Some("Check passed".to_string()),
                        metrics: None,
                    }
                } else {
                    all_healthy = false;
                    
                    HealthCheckResult {
                        status: HealthStatus::Unhealthy,
                        details: Some("Check failed".to_string()),
                        metrics: None,
                    }
                };
                
                results.insert(name, result);
            }
        }
        
        // Run async checks
        for name in async_check_names {
            let check_fn = {
                let checks = self.async_checks.read().await;
                checks.get(&name).cloned()
            };
            
            if let Some(check) = check_fn {
                let result = check().await;
                
                if result.status == HealthStatus::Healthy {
                    all_healthy &= true;
                    any_healthy |= true;
                } else {
                    all_healthy = false;
                    if result.status == HealthStatus::Degraded {
                        any_healthy |= true;
                    }
                }
                
                results.insert(name, result);
            }
        }
        
        let status = if all_healthy {
            HealthStatus::Healthy
        } else if any_healthy {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        };
        
        HealthResponse {
            status,
            checks: results,
            uptime: self.uptime(),
        }
    }
} 
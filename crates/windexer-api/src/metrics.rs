use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;

/// Metrics service for collecting and retrieving metrics
#[derive(Debug)]
pub struct MetricsService {
    /// Stored metrics
    metrics: Arc<RwLock<HashMap<String, Value>>>,
}

impl MetricsService {
    /// Create a new metrics service
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set a metric value
    pub async fn set_metric(&self, key: &str, value: Value) {
        let mut metrics = self.metrics.write().await;
        metrics.insert(key.to_string(), value);
    }

    /// Get a specific metric value
    pub async fn get_metric(&self, key: &str) -> Option<Value> {
        let metrics = self.metrics.read().await;
        metrics.get(key).cloned()
    }

    /// Remove a metric
    pub async fn remove_metric(&self, key: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.remove(key);
    }

    /// Get all metrics
    pub async fn get_metrics(&self) -> Value {
        let metrics = self.metrics.read().await;
        serde_json::to_value(metrics.clone()).unwrap_or(Value::Object(serde_json::Map::new()))
    }

    /// Register a function to update metrics periodically
    pub fn register_collector<F>(&self, collector: F)
    where
        F: Fn() -> HashMap<String, Value> + Send + Sync + 'static,
    {
        let metrics = self.metrics.clone();
        
        tokio::spawn(async move {
            loop {
                let collected = collector();
                
                let mut metrics_lock = metrics.write().await;
                for (key, value) in collected {
                    metrics_lock.insert(key, value);
                }
                
                // Update every 10 seconds
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            }
        });
    }
} 
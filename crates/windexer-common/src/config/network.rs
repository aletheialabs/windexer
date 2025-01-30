use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Network-specific configuration
    pub listen_addresses: Vec<String>,
    pub bootstrap_peers: Vec<String>,
    pub heartbeat_interval: u64,
    pub connection_timeout: u64,
    pub max_peers: usize,
}

impl NetworkConfig {
    pub fn heartbeat_duration(&self) -> Duration {
        Duration::from_millis(self.heartbeat_interval)
    }

    pub fn connection_timeout_duration(&self) -> Duration {
        Duration::from_millis(self.connection_timeout)
    }
}
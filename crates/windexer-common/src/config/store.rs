use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreConfig {
    /// Storage configuration
    pub database_url: String,
    pub max_connections: u32,
    pub cache_capacity: usize,
    pub sync_interval: u64,
}
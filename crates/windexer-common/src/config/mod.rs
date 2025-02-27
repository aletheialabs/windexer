//! Configuration types for the wIndexer system

mod network;
mod store;
pub mod node;

pub use network::NetworkConfig;
pub use store::StoreConfig;
pub use node::{NodeType, NodeConfig, PublisherNodeConfig, RelayerNodeConfig};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerConfig {
    /// Global configuration for the indexer
    pub data_dir: PathBuf,
    pub network: NetworkConfig,
    pub store: StoreConfig,
    pub log_level: String,
    pub metrics_enabled: bool,
}

impl IndexerConfig {
    pub fn from_file(path: &str) -> crate::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&contents)?)
    }
}
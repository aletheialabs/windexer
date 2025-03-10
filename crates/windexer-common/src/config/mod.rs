//! Configuration types for the wIndexer system

mod network;
mod store;
pub mod node;

// Comment out these imports to resolve duplicates
// pub use network::NetworkConfig;
// pub use store::StoreConfig;
pub use node::NodeConfig;

use {
    std::{fs, path::{Path, PathBuf}},
    serde::{Deserialize, Serialize},
};

use crate::errors::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerConfig {
    pub data_dir: PathBuf,
    pub network: NetworkConfig,
    pub store: StoreConfig,
    pub log_level: String,
    pub metrics_enabled: bool,
    pub geyser: Option<GeyserConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub bind_address: String,
    pub peers: Vec<String>,
    pub bootstrap_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreConfig {
    pub db_path: String,
    pub max_size_gb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeyserConfig {
    pub validator_url: String,
    pub libpath: String,
    pub config_file: String,
}

impl IndexerConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        Ok(serde_json::from_str::<Self>(&contents)?)
    }
}
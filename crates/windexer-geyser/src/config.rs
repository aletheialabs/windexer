// crates/windexer-geyser/src/config.rs

//! Plugin configuration handling
//!
//! This module contains the configuration types and parsing logic for the wIndexer Geyser plugin.

use {
    agave_geyser_plugin_interface::geyser_plugin_interface::{
        GeyserPluginError, Result as PluginResult,
    },
    serde::{Deserialize, Serialize},
    anyhow::{anyhow, Result},
    std::{
        fs::File,
        io::Read,
        net::SocketAddr,
        path::Path,
        str::FromStr,
    },
    solana_sdk::{
        pubkey::Pubkey,
        signature::Keypair,
    },
    windexer_common,
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AccountsSelector {
    pub accounts: Vec<String>,
    #[serde(default)]
    pub owners: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TransactionSelector {
    pub mentions: Vec<String>,
    #[serde(default)]
    pub include_votes: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NetworkConfig {
    pub node_id: String,
    pub listen_addr: SocketAddr,
    pub rpc_addr: SocketAddr,
    pub bootstrap_peers: Vec<String>,
    pub data_dir: String,
    pub solana_rpc_url: String,
    #[serde(default)]
    pub geyser_plugin_config: Option<String>,
    #[serde(default)]
    pub metrics_addr: Option<SocketAddr>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MetricsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_metrics_interval")]
    pub interval_seconds: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum StorageType {
    #[serde(rename = "rocksdb")]
    RocksDB,
    #[serde(rename = "parquet")]
    Parquet,
    #[serde(rename = "postgres")]
    Postgres,
}

impl Default for StorageType {
    fn default() -> Self {
        StorageType::RocksDB
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ParquetConfig {
    pub directory: String,
    #[serde(default = "default_parquet_file_size_mb")]
    pub max_file_size_mb: usize,
    #[serde(default = "default_true")]
    pub compression_enabled: bool,
    #[serde(default = "default_parquet_partition_by_slot")]
    pub partition_by_slot: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PostgresConfig {
    pub connection_string: String,
    #[serde(default = "default_true")]
    pub create_tables: bool,
    #[serde(default = "default_postgres_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_postgres_max_connections")]
    pub max_connections: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StorageConfig {
    #[serde(default)]
    pub storage_type: StorageType,
    #[serde(default)]
    pub parquet: Option<ParquetConfig>,
    #[serde(default)]
    pub postgres: Option<PostgresConfig>,
    #[serde(default)]
    pub rocksdb_path: Option<String>,
    #[serde(default = "default_true")]
    pub hot_cold_separation: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            storage_type: StorageType::RocksDB,
            parquet: None,
            postgres: None,
            rocksdb_path: None,
            hot_cold_separation: true,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeyserPluginConfig {
    pub libpath: String,
    pub keypair: String, // Path to keypair file
    #[serde(default)]
    pub host: Option<String>,
    pub network: NetworkConfig,
    #[serde(default)]
    pub accounts_selector: Option<AccountsSelector>,
    #[serde(default)]
    pub transaction_selector: Option<TransactionSelector>,
    #[serde(default = "default_thread_count")]
    pub thread_count: usize,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default)]
    pub node_pubkey: Option<String>,
    #[serde(default)]
    pub panic_on_error: bool,
    #[serde(default = "default_true")]
    pub use_mmap: bool,
    #[serde(default)]
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub storage: StorageConfig,
}

// Simplified SerializableKeypair - only implements what we need
pub struct SerializableKeypair(Keypair);

impl SerializableKeypair {
    pub fn new(keypair: Keypair) -> Self {
        Self(keypair)
    }
    
    pub fn inner(&self) -> &Keypair {
        &self.0
    }
}

impl Default for SerializableKeypair {
    fn default() -> Self {
        Self(Keypair::new())
    }
}

impl Clone for SerializableKeypair {
    fn clone(&self) -> Self {
        let bytes = self.0.to_bytes();
        Self(Keypair::from_bytes(&bytes).expect("Valid keypair bytes"))
    }
}

impl GeyserPluginConfig {
    pub fn load_from_file<P: AsRef<Path>>(file_path: P) -> Result<Self, GeyserPluginError> {
        let mut file = File::open(&file_path).map_err(|err| {
            GeyserPluginError::ConfigFileOpenError(err)
        })?;
        
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|err| {
            GeyserPluginError::ConfigFileReadError { msg: format!("Failed to read config file: {}", err) }
        })?;
        
        Self::load_from_str(&contents)
    }
    
    pub fn load_from_str(config_str: &str) -> Result<Self, GeyserPluginError> {
        serde_json::from_str(config_str).map_err(|err| {
            GeyserPluginError::ConfigFileReadError { 
                msg: format!("Failed to parse config file: {}", err) 
            }
        })
    }
    
    // Add the missing validate method
    pub fn validate(&self) -> Result<(), String> {
        // Basic validation checks
        if self.libpath.is_empty() {
            return Err("libpath cannot be empty".to_string());
        }
        if self.keypair.is_empty() {
            return Err("keypair cannot be empty".to_string());
        }
        Ok(())
    }
    
    // Helper method to get accounts selector or default
    pub fn get_accounts_selector(&self) -> AccountsSelector {
        self.accounts_selector.clone().unwrap_or_else(|| AccountsSelector {
            accounts: vec!["*".to_string()],
            owners: None,
        })
    }
    
    // Helper method to get transaction selector or default
    pub fn get_transaction_selector(&self) -> TransactionSelector {
        self.transaction_selector.clone().unwrap_or_else(|| TransactionSelector {
            mentions: vec!["*".to_string()],
            include_votes: false,
        })
    }
    
    // Load keypair from file path - simplified to reduce dependencies
    pub fn load_keypair(&self) -> Result<Keypair, GeyserPluginError> {
        let keypair_bytes = std::fs::read(&self.keypair).map_err(|err| {
            GeyserPluginError::ConfigFileReadError { 
                msg: format!("Failed to read keypair file: {}", err) 
            }
        })?;
        
        Keypair::from_bytes(&keypair_bytes).map_err(|err| {
            GeyserPluginError::ConfigFileReadError { 
                msg: format!("Invalid keypair file format: {}", err) 
            }
        })
    }
}

fn default_thread_count() -> usize {
    4
}

fn default_batch_size() -> usize {
    100
}

fn default_true() -> bool {
    true
}

fn default_metrics_interval() -> u64 {
    15
}

fn default_parquet_file_size_mb() -> usize {
    128 // 128 MB per file is a good balance for Parquet
}

fn default_parquet_partition_by_slot() -> bool {
    true // Partitioning by slot is efficient for blockchain data
}

fn default_postgres_batch_size() -> usize {
    1000 // Default batch size for PostgreSQL inserts
}

fn default_postgres_max_connections() -> usize {
    20 // Default connection pool size for PostgreSQL
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            interval_seconds: default_metrics_interval(),
        }
    }
}

impl Default for GeyserPluginConfig {
    fn default() -> Self {
        Self {
            libpath: "".to_string(),
            keypair: "".to_string(),
            host: None,
            network: NetworkConfig {
                node_id: "windexer-node".to_string(),
                listen_addr: "127.0.0.1:8900".parse().unwrap(),
                rpc_addr: "127.0.0.1:8901".parse().unwrap(),
                bootstrap_peers: vec![],
                data_dir: "/tmp/windexer".to_string(),
                solana_rpc_url: "http://127.0.0.1:8899".to_string(),
                geyser_plugin_config: None,
                metrics_addr: None,
            },
            accounts_selector: None,
            transaction_selector: None,
            thread_count: 4,
            batch_size: 100,
            node_pubkey: None,
            panic_on_error: false,
            use_mmap: true,
            metrics: MetricsConfig::default(),
            storage: StorageConfig::default(),
        }
    }
}
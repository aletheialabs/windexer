//! Plugin configuration handling
//!
//! This module contains the configuration types and parsing logic for the wIndexer Geyser plugin.

use {
    serde::{Deserialize, Serialize},
    anyhow::{anyhow, Result},
    std::{
        fs::File,
        io::Read,
        path::Path,
        str::FromStr,
    },
    solana_sdk::{
        pubkey::Pubkey,
        signer::keypair::Keypair,
    },
    windexer_network::NodeConfig as NetworkConfig,
    bs58,
    windexer_common,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct GeyserPluginConfig {
    pub libpath: String,
    
    pub network: NetworkConfig,
    
    pub accounts_selector: Option<AccountsSelector>,
    
    pub transaction_selector: Option<TransactionSelector>,
    
    #[serde(default = "default_thread_count")]
    pub thread_count: usize,
    
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    
    #[serde(default)]
    pub panic_on_error: bool,
    
    #[serde(default = "default_true")]
    pub use_mmap: bool,
    
    pub node_pubkey: Option<String>,
    
    #[serde(default)]
    pub metrics: MetricsConfig,
    
    #[serde(with = "keypair_serde")]
    pub keypair: Keypair,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountsSelector {
    pub accounts: Option<Vec<String>>,
    
    pub owners: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionSelector {
    pub mentions: Vec<String>,
    
    #[serde(default)]
    pub include_votes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    #[serde(default = "default_metrics_interval")]
    pub interval_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializableKeypair(#[serde(with = "keypair_serde")] pub Keypair);

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

mod keypair_serde {
    use {
        serde::{Deserialize, Deserializer, Serialize, Serializer},
        solana_sdk::signer::keypair::Keypair,
        bs58,
    };

    pub fn serialize<S>(keypair: &Keypair, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = bs58::encode(&keypair.to_bytes()).into_string();
        s.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Keypair, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = bs58::decode(&s).into_vec().map_err(serde::de::Error::custom)?;
        Keypair::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

impl GeyserPluginConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(&path)
            .map_err(|e| anyhow!("Failed to open config file: {}", e))?;
        
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| anyhow!("Failed to read config file: {}", e))?;
        
        serde_json::from_str(&contents)
            .map_err(|e| anyhow!("Failed to parse config file: {}", e))
    }
    
    pub fn validate(&self) -> Result<()> {
        if let Some(selector) = &self.accounts_selector {
            if let Some(accounts) = &selector.accounts {
                for account in accounts {
                    if account != "*" {
                        Pubkey::from_str(account)
                            .map_err(|_| anyhow!("Invalid account pubkey: {}", account))?;
                    }
                }
            }
            
            if let Some(owners) = &selector.owners {
                for owner in owners {
                    if owner == "*" {
                    } else {
                        Pubkey::from_str(owner)
                            .map_err(|_| anyhow!("Invalid owner pubkey: {}", owner))?;
                    }
                }
            }
        }
        
        if let Some(selector) = &self.transaction_selector {
            for mention in &selector.mentions {
                if mention != "*" && mention != "all_votes" {
                    Pubkey::from_str(mention)
                        .map_err(|_| anyhow!("Invalid mention pubkey: {}", mention))?;
                }
            }
        }
        
        Ok(())
    }
}

fn default_thread_count() -> usize {
    num_cpus::get().max(1)
}

fn default_batch_size() -> usize {
    1000
}

fn default_true() -> bool {
    true
}

fn default_metrics_interval() -> u64 {
    60
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
            network: NetworkConfig {
                node_id: "windexer-node".to_string(),
                listen_addr: "127.0.0.1:8900".parse().unwrap(),
                rpc_addr: "127.0.0.1:8901".parse().unwrap(),
                bootstrap_peers: vec![],
                data_dir: "/tmp/windexer".to_string(),
                solana_rpc_url: "http://127.0.0.1:8899".to_string(),
                keypair: windexer_common::SerializableKeypair::default(),
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
            keypair: Keypair::new(),
        }
    }
}

impl Clone for GeyserPluginConfig {
    fn clone(&self) -> Self {
        Self {
            libpath: self.libpath.clone(),
            network: self.network.clone(),
            accounts_selector: self.accounts_selector.clone(),
            transaction_selector: self.transaction_selector.clone(),
            thread_count: self.thread_count,
            batch_size: self.batch_size,
            panic_on_error: self.panic_on_error,
            use_mmap: self.use_mmap,
            node_pubkey: self.node_pubkey.clone(),
            metrics: self.metrics.clone(),
            keypair: Keypair::new(),
        }
    }
}
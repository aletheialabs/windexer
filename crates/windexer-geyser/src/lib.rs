//! wIndexer Geyser Plugin for agave
//!
//! A high-performance agave Geyser plugin that streams data to the wIndexer 
//! decentralized network using advanced techniques like memory mapping, SIMD processing,
//! and p2p propagation.

use std::sync::{Arc, RwLock};
use std::fmt::Debug;

use agave_geyser_plugin_interface::geyser_plugin_interface::{
    GeyserPlugin, GeyserPluginError, ReplicaAccountInfoVersions, ReplicaBlockInfoVersions,
    ReplicaTransactionInfoVersions, ReplicaEntryInfoVersions, Result as PluginResult, SlotStatus,
};

mod memory_mapped;
mod p2p_propagator;
mod simd_processing;
mod validator_integration;

use memory_mapped::MemoryMappedAccounts;
use p2p_propagator::P2PPropagator;
use simd_processing::SimdProcessor;
use validator_integration::ValidatorIntegration;

include!(concat!(env!("OUT_DIR"), "/build_info.rs"));

#[derive(Debug, Default)]
pub struct PluginConfig {
    pub mmap_path: Option<String>,
    pub mmap_size: usize,
    pub bootstrap_nodes: Vec<String>,
    pub enable_simd: bool,
    pub enable_validator_integration: bool,
    pub validator_rpc_url: Option<String>,
    pub p2p_topics: Vec<String>,
    pub data_dir: Option<String>,
}

impl PluginConfig {
    pub fn from_json(config: &serde_json::Value) -> Result<Self, GeyserPluginError> {
        let mmap_path = config.get("mmap_path")
            .and_then(|v| v.as_str())
            .map(String::from);
        
        let mmap_size = config.get("mmap_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(1024 * 1024 * 1024) as usize;
        
        let bootstrap_nodes = config.get("bootstrap_nodes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        
        let enable_simd = config.get("enable_simd")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        
        let enable_validator_integration = config.get("enable_validator_integration")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let validator_rpc_url = config.get("validator_rpc_url")
            .and_then(|v| v.as_str())
            .map(String::from);
            
        let p2p_topics = config.get("p2p_topics")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_else(|| vec![
                "accounts".to_string(),
                "transactions".to_string(),
                "blocks".to_string(),
                "slots".to_string(),
            ]);
            
        let data_dir = config.get("data_dir")
            .and_then(|v| v.as_str())
            .map(String::from);
        
        Ok(Self {
            mmap_path,
            mmap_size,
            bootstrap_nodes,
            enable_simd,
            enable_validator_integration,
            validator_rpc_url,
            p2p_topics,
            data_dir,
        })
    }
}

#[derive(Debug)]
pub struct WindexerGeyserPlugin {
    config: Arc<RwLock<PluginConfig>>,
    memory_mapped: Option<Arc<MemoryMappedAccounts>>,
    propagator: Option<Arc<P2PPropagator>>,
    simd_processor: Option<Arc<SimdProcessor>>,
    validator_integration: Option<Arc<ValidatorIntegration>>,
}

impl WindexerGeyserPlugin {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(PluginConfig::default())),
            memory_mapped: None,
            propagator: None,
            simd_processor: None,
            validator_integration: None,
        }
    }
}

impl GeyserPlugin for WindexerGeyserPlugin {
    fn name(&self) -> &'static str {
        "windexer-geyser"
    }

    fn on_load(&mut self, config_file: &str, _is_reload: bool) -> PluginResult<()> {
        tracing::info!("Loading wIndexer Geyser plugin (build: {})", BUILD_DATE);
        
        let config_content = std::fs::read_to_string(config_file)
            .map_err(|e| GeyserPluginError::ConfigFileReadError {
                msg: format!("Failed to read config file {}: {}", config_file, e),
            })?;

        let config_json: serde_json::Value = serde_json::from_str(&config_content)
            .map_err(|e| GeyserPluginError::ConfigFileReadError {
                msg: format!("Failed to parse config file as JSON: {}", e),
            })?;

        let plugin_config = PluginConfig::from_json(&config_json)?;
        *self.config.write().unwrap() = plugin_config;

        let config = self.config.read().unwrap().clone();
        
        if let Some(mmap_path) = &config.mmap_path {
            tracing::info!("Initializing memory mapping from {}", mmap_path);
            self.memory_mapped = Some(Arc::new(MemoryMappedAccounts::new(
                mmap_path,
                config.mmap_size,
            )?));
        }
        
        if config.enable_simd {
            tracing::info!("Initializing SIMD processor");
            self.simd_processor = Some(Arc::new(SimdProcessor::new(config.enable_simd)?));
        }
        
        if config.enable_validator_integration {
            tracing::info!("Initializing validator integration");
            self.validator_integration = Some(Arc::new(ValidatorIntegration::new(
                config.validator_rpc_url.as_deref(),
            )?));
        }
        
        tracing::info!("Initializing p2p propagator");
        self.propagator = Some(Arc::new(P2PPropagator::new(
            &config.bootstrap_nodes,
            &config.p2p_topics,
            config.data_dir.as_deref(),
        )?));

        tracing::info!("wIndexer Geyser plugin loaded successfully");
        Ok(())
    }

    fn on_unload(&mut self) {
        tracing::info!("Unloading wIndexer Geyser plugin");
        
        self.propagator = None;
        self.validator_integration = None;
        self.simd_processor = None;
        self.memory_mapped = None;
        
        tracing::info!("wIndexer Geyser plugin unloaded");
    }

    fn update_account(
        &self,
        account: ReplicaAccountInfoVersions,
        slot: u64,
        is_startup: bool,
    ) -> PluginResult<()> {
        if is_startup && self.memory_mapped.is_some() {
            if let Some(mmap) = &self.memory_mapped {
                mmap.store_account(&account, slot)?;
            }
            return Ok(());
        }

        let processed_data = if let Some(processor) = &self.simd_processor {
            processor.process_account(account.clone(), slot)?
        } else {
            bincode::serialize(&(slot, account)).map_err(|e| GeyserPluginError::AccountsUpdateError {
                msg: format!("Failed to serialize account: {}", e),
            })?
        };
        
        if let Some(propagator) = &self.propagator {
            propagator.propagate_account(processed_data, slot)?;
        }
        
        if let Some(mmap) = &self.memory_mapped {
            mmap.store_account(&account, slot)?;
        }
        
        Ok(())
    }

    fn notify_end_of_startup(&self) -> PluginResult<()> {
        tracing::info!("End of startup notification received");
        
        if let Some(mmap) = &self.memory_mapped {
            mmap.flush()?;
        }
        
        Ok(())
    }

    fn update_slot_status(
        &self,
        slot: u64,
        parent: Option<u64>,
        status: SlotStatus,
    ) -> PluginResult<()> {
        if let Some(processor) = &self.simd_processor {
            let processed_data = processor.process_slot(slot, parent, status)?;
            
            if let Some(propagator) = &self.propagator {
                propagator.propagate_slot(processed_data, slot)?;
            }
        } else {
            let slot_data = (slot, parent, status);
            let processed_data = bincode::serialize(&slot_data)
                .map_err(|e| GeyserPluginError::SlotStatusUpdateError {
                    msg: format!("Failed to serialize slot status: {}", e),
                })?;
                
            if let Some(propagator) = &self.propagator {
                propagator.propagate_slot(processed_data, slot)?;
            }
        }
        
        Ok(())
    }

    fn notify_transaction(
        &self,
        transaction: ReplicaTransactionInfoVersions,
        slot: u64,
    ) -> PluginResult<()> {
        if let Some(processor) = &self.simd_processor {
            let processed_data = processor.process_transaction(transaction.clone(), slot)?;
            
            if let Some(propagator) = &self.propagator {
                propagator.propagate_transaction(processed_data, slot)?;
            }
        } else {
            let tx_data = (slot, transaction);
            let processed_data = bincode::serialize(&tx_data)
                .map_err(|e| GeyserPluginError::TransactionUpdateError {
                    msg: format!("Failed to serialize transaction: {}", e),
                })?;
                
            if let Some(propagator) = &self.propagator {
                propagator.propagate_transaction(processed_data, slot)?;
            }
        }
        
        Ok(())
    }

    fn notify_block_metadata(
        &self,
        blockinfo: ReplicaBlockInfoVersions,
    ) -> PluginResult<()> {
        if let Some(processor) = &self.simd_processor {
            let processed_data = processor.process_block(blockinfo.clone())?;
            
            if let Some(propagator) = &self.propagator {
                propagator.propagate_block(processed_data)?;
            }
        } else {
            let processed_data = bincode::serialize(&blockinfo)
                .map_err(|e| GeyserPluginError::AccountsUpdateError {
                    msg: format!("Failed to serialize block metadata: {}", e),
                })?;
                
            if let Some(propagator) = &self.propagator {
                propagator.propagate_block(processed_data)?;
            }
        }
        
        Ok(())
    }

    fn notify_entry(&self, entry: ReplicaEntryInfoVersions) -> PluginResult<()> {
        if let Some(processor) = &self.simd_processor {
            let processed_data = processor.process_entry(entry.clone())?;
            
            if let Some(propagator) = &self.propagator {
                propagator.propagate_entry(processed_data)?;
            }
        } else {
            let processed_data = bincode::serialize(&entry)
                .map_err(|e| GeyserPluginError::AccountsUpdateError {
                    msg: format!("Failed to serialize entry: {}", e),
                })?;
                
            if let Some(propagator) = &self.propagator {
                propagator.propagate_entry(processed_data)?;
            }
        }
        
        Ok(())
    }

    fn account_data_notifications_enabled(&self) -> bool {
        true
    }

    fn transaction_notifications_enabled(&self) -> bool {
        true
    }

    fn entry_notifications_enabled(&self) -> bool {
        true
    }
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub unsafe extern "C" fn _create_plugin() -> *mut dyn GeyserPlugin {
    let plugin = WindexerGeyserPlugin::new();
    let boxed = Box::new(plugin);
    Box::into_raw(boxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_name() {
        let plugin = WindexerGeyserPlugin::new();
        assert_eq!(plugin.name(), "windexer-geyser");
    }

    #[test]
    fn test_plugin_config_from_json() {
        let config_json = serde_json::json!({
            "mmap_path": "/tmp/windexer-accounts.mmap",
            "mmap_size": 2147483648,
            "bootstrap_nodes": [
                "/ip4/127.0.0.1/tcp/8900/p2p/QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N",
                "/ip4/127.0.0.1/tcp/8901/p2p/QmaBvfZooxWkrv7D3r8LS9moNjzD2o525XMH93Hzg5GRAu"
            ],
            "enable_simd": true,
            "enable_validator_integration": false,
            "validator_rpc_url": "http://localhost:8899",
            "p2p_topics": ["accounts", "transactions", "blocks"],
            "data_dir": "/var/lib/windexer"
        });

        let config = PluginConfig::from_json(&config_json).unwrap();
        
        assert_eq!(config.mmap_path, Some("/tmp/windexer-accounts.mmap".to_string()));
        assert_eq!(config.mmap_size, 2147483648);
        assert_eq!(config.bootstrap_nodes.len(), 2);
        assert_eq!(config.enable_simd, true);
        assert_eq!(config.enable_validator_integration, false);
        assert_eq!(config.validator_rpc_url, Some("http://localhost:8899".to_string()));
        assert_eq!(config.p2p_topics.len(), 3);
        assert_eq!(config.data_dir, Some("/var/lib/windexer".to_string()));
    }
}

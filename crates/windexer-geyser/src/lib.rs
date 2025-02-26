// crates/windexer-geyser/src/lib.rs
#![allow(clippy::unsafe_derive_deserialize)]

use {
    memory_mapped::ValidatorMemoryMap,
    p2p_propagator::TidePropagator,
    simd_processing::SimdProcessor,
    solana_geyser_plugin_interface::{
        geyser_plugin_interface::{GeyserPlugin, ReplicaAccountInfo, Result as GeyserResult},
        ReplicaAccountInfoVersions,
    },
    std::{
        ffi::CStr,
        os::raw::c_char,
        path::Path,
        sync::{Arc, Mutex},
        time::{SystemTime, UNIX_EPOCH},
    },
    validator_integration::{TideHook, ValidatorIntegrationError},
    windexer_common::{
        config::{Config, TideConfig},
        crypto::{BLSKeypair, Ed25519Keypair},
        types::{TideBatch, TideHeader},
    },
};

mod memory_mapped;
mod p2p_propagator;
mod simd_processing;
mod validator_integration;

/// Main Tide Geyser Plugin implementation
pub struct TideGeyser {
    config: TideConfig,
    memory_map: Option<Arc<Mutex<ValidatorMemoryMap>>>,
    tide_hook: Option<TideHook>,
    propagator: TidePropagator,
    bls_keypair: BLSKeypair,
    ed25519_keypair: Ed25519Keypair,
    running: bool,
}

impl GeyserPlugin for TideGeyser {
    fn name(&self) -> &'static str {
        "Windexer Tide Geyser Plugin"
    }

    fn on_load(
        &mut self,
        config_file: &str,
    ) -> GeyserResult<()> {
        let config = Config::load(Path::new(config_file))?;
        self.config = config.tide.clone().ok_or_else(|| {
            solana_geyser_plugin_interface::GeyserPluginError::ConfigParsingError {
                msg: "Missing Tide config section".to_string(),
            }
        })?;

        // Initialize cryptography
        self.bls_keypair = BLSKeypair::from_file(&self.config.bls_key_path)?;
        self.ed25519_keypair = Ed25519Keypair::from_file(&self.config.ed25519_key_path)?;

        // Try to establish direct memory mapping
        if let Ok(mmap) = unsafe {
            ValidatorMemoryMap::new(
                self.config.validator_pid,
                self.config.accounts_start_address,
                self.config.accounts_region_size,
            )
        } {
            self.memory_map = Some(Arc::new(Mutex::new(mmap)));
            log::info!("Direct memory mapping established");
        } else {
            log::warn!("Falling back to standard Geyser plugin mode");
        }

        // Initialize P2P propagator
        self.propagator = TidePropagator::new(
            self.config.gossip.clone(),
            self.config.network_metrics.clone(),
            self.config.chain_id,
        )?;

        Ok(())
    }

    fn update_account(
        &mut self,
        account: ReplicaAccountInfoVersions<'_>,
        slot: u64,
        is_startup: bool,
    ) -> GeyserResult<()> {
        let account_info = match account {
            ReplicaAccountInfoVersions::V0_0_1(acc) => acc,
        };

        // Process via direct memory mapping if available
        if let Some(mmap) = &self.memory_map {
            let mmap_lock = mmap.lock().unwrap();
            let account_data = unsafe { mmap_lock.get_account(account_info.pubkey as usize) }?;
            
            // SIMD process batch
            let processed = SimdProcessor::process_accounts(&[account_data.clone()])?;
            
            // Create Tide batch
            let batch = TideBatch {
                header: TideHeader {
                    slot,
                    parent_slot: slot.saturating_sub(1),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64,
                    chain_id: self.config.chain_id,
                },
                data: processed,
            };

            // Propagate through P2P network
            self.propagator.propagate(batch)?;
        } else {
            // Fallback to traditional Geyser processing
            let account = convert_account_info(account_info)?;
            let processed = SimdProcessor::process_accounts(&[account])?;
            
            let batch = TideBatch {
                header: TideHeader {
                    slot,
                    parent_slot: slot.saturating_sub(1),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64,
                    chain_id: self.config.chain_id,
                },
                data: processed,
            };

            self.propagator.propagate(batch)?;
        }

        Ok(())
    }

    fn on_unload(&mut self) {
        if let Some(hook) = self.tide_hook.take() {
            hook.shutdown();
        }
        self.running = false;
        log::info!("Tide Geyser plugin unloaded");
    }

    fn account_data_notifications_enabled(&self) -> bool {
        true
    }

    fn transaction_notifications_enabled(&self) -> bool {
        self.config.enable_transaction_notifications
    }
}

impl TideGeyser {
    /// Initialize the Tide Geyser plugin with validator integration
    pub fn new() -> Self {
        Self {
            config: TideConfig::default(),
            memory_map: None,
            tide_hook: None,
            propagator: TidePropagator::default(),
            bls_keypair: BLSKeypair::default(),
            ed25519_keypair: Ed25519Keypair::default(),
            running: false,
        }
    }

    /// Start validator pipeline taps
    pub fn start_taps(
        &mut self,
        bank_forks: solana_runtime::bank_forks::BankForks,
        block_commitment_cache: solana_runtime::block_commitment_cache::BlockCommitmentCache,
    ) -> Result<(), ValidatorIntegrationError> {
        let hook = TideHook::new(
            vec![
                solana_runtime::pipeline::PipelineStage::TPUReceive,
                solana_runtime::pipeline::PipelineStage::TVUVote,
            ],
            Arc::new(bank_forks),
            Arc::new(block_commitment_cache),
        );
        
        hook.install_all()?;
        self.tide_hook = Some(hook);
        self.running = true;
        Ok(())
    }

    /// Get current propagation metrics
    pub fn metrics(&self) -> std::collections::BTreeMap<String, u64> {
        self.propagator.metrics()
    }
}

/// Convert raw account info to Solana SDK Account
fn convert_account_info(
    info: &ReplicaAccountInfo,
) -> Result<solana_sdk::account::Account, Box<dyn std::error::Error>> {
    Ok(solana_sdk::account::Account {
        lamports: info.lamports,
        data: info.data[0..info.data_len as usize].to_vec(),
        owner: Pubkey::try_from(info.owner).unwrap(),
        executable: info.executable,
        rent_epoch: info.rent_epoch,
    })
}

/// Required C interface for Geyser plugins
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn _create_plugin() -> *mut dyn GeyserPlugin {
    let plugin = TideGeyser::new();
    let boxed = Box::new(plugin);
    Box::into_raw(boxed)
}

/// Default implementations for FFI
impl Default for TideGeyser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_geyser_plugin_interface::geyser_plugin_interface::ReplicaAccountInfo;

    fn test_account_info() -> ReplicaAccountInfoVersions<'_> {
        let mut info = ReplicaAccountInfo {
            pubkey: [1; 32],
            lamports: 100,
            owner: [2; 32],
            executable: false,
            rent_epoch: 0,
            data: vec![0; 128].as_mut_ptr(),
            data_len: 128,
            write_version: 0,
        };
        ReplicaAccountInfoVersions::V0_0_1(&mut info)
    }

    #[test]
    fn test_plugin_creation() {
        let mut plugin = TideGeyser::new();
        let config_path = format!("{}/test_config.toml", env!("CARGO_MANIFEST_DIR"));
        
        assert!(plugin.on_load(&config_path).is_ok());
        assert!(plugin.update_account(test_account_info(), 1, false).is_ok());
        plugin.on_unload();
    }

    #[test]
    fn test_metrics_collection() {
        let plugin = TideGeyser::new();
        let metrics = plugin.metrics();
        assert!(metrics.contains_key("high_priority"));
    }
}

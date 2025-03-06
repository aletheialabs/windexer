//! Validator integration for direct access to validator internals
//!
//! This module provides integration with the agave validator beyond the standard
//! Geyser plugin interface. It allows for direct access to validator TPU pipeline,
//! bank state, and other validator-specific optimizations.

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use solana_client::rpc_client::RpcClient;
use agave_geyser_plugin_interface::geyser_plugin_interface::{
    GeyserPluginError, Result as PluginResult,
};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::Transaction;

const MAX_RPC_RETRIES: u32 = 5;

const RPC_TIMEOUT_SECONDS: u64 = 30;

const TPU_FORWARD_INTERVAL_MS: u64 = 200;

#[derive(Debug)]
pub struct ValidatorIntegration {
    rpc_client: Option<Arc<RpcClient>>,
    
    known_validators: Arc<RwLock<Vec<Pubkey>>>,
    
    tpu_forwarding_enabled: bool,
    
    last_tpu_forward: Arc<RwLock<Instant>>,
    
    current_leader_slot: Arc<RwLock<Option<u64>>>,
    
    current_leader: Arc<RwLock<Option<Pubkey>>>,
}

/// Information about a TPU tap point
#[derive(Debug, Clone, PartialEq)]
pub enum TpuTapPoint {
    /// TPU receive stage (earliest, unvalidated)
    Receive,
    
    /// TPU forward stage (signature verified)
    Forward,
    
    /// TVU vote stage (PoH verified)
    Vote,
    
    /// Bank commit stage (finalized)
    BankCommit,
}

impl ValidatorIntegration {
    /// Create a new validator integration
    pub fn new(rpc_url: Option<&str>) -> PluginResult<Self> {
        // Create RPC client if URL provided
        let rpc_client = match rpc_url {
            Some(url) => {
                tracing::info!("Initializing validator integration with RPC URL: {}", url);
                
                // Configure RPC client with appropriate timeout
                let timeout = Duration::from_secs(RPC_TIMEOUT_SECONDS);
                let rpc_client = RpcClient::new_with_timeout_and_commitment(
                    url.to_string(),
                    timeout,
                    CommitmentConfig::confirmed(),
                );
                
                // Test connection to ensure RPC is accessible
                match rpc_client.get_version() {
                    Ok(version) => {
                        tracing::info!("Connected to Solana validator version: {}", version.solana_core);
                        Some(Arc::new(rpc_client))
                    }
                    Err(err) => {
                        tracing::warn!("Failed to connect to RPC at {}: {}", url, err);
                        None
                    }
                }
            }
            None => {
                tracing::info!("No RPC URL provided, validator integration will operate in limited mode");
                None
            }
        };

        Ok(Self {
            rpc_client,
            known_validators: Arc::new(RwLock::new(Vec::new())),
            tpu_forwarding_enabled: false,
            last_tpu_forward: Arc::new(RwLock::new(Instant::now())),
            current_leader_slot: Arc::new(RwLock::new(None)),
            current_leader: Arc::new(RwLock::new(None)),
        })
    }

    /// Enable TPU forwarding
    pub fn enable_tpu_forwarding(&mut self) -> PluginResult<()> {
        if self.rpc_client.is_none() {
            return Err(GeyserPluginError::Custom(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Cannot enable TPU forwarding without RPC connection",
                ),
            )));
        }

        self.tpu_forwarding_enabled = true;
        tracing::info!("TPU forwarding enabled");
        Ok(())
    }

    /// Disable TPU forwarding
    pub fn disable_tpu_forwarding(&mut self) {
        self.tpu_forwarding_enabled = false;
        tracing::info!("TPU forwarding disabled");
    }

    /// Connect to a TPU tap point
    pub fn connect_tpu_tap(&self, tap_point: TpuTapPoint) -> PluginResult<()> {
        // This would require direct validator integration at binary level
        // For now, we just log the request as this is not possible through a plugin
        match tap_point {
            TpuTapPoint::Receive => {
                tracing::info!("Requested TPU receive tap point (earliest, unvalidated)");
            }
            TpuTapPoint::Forward => {
                tracing::info!("Requested TPU forward tap point (signature verified)");
            }
            TpuTapPoint::Vote => {
                tracing::info!("Requested TVU vote tap point (PoH verified)");
            }
            TpuTapPoint::BankCommit => {
                tracing::info!("Requested bank commit tap point (finalized)");
            }
        }

        tracing::warn!("Direct TPU tapping is not currently supported through the plugin interface");
        tracing::info!("Use memory mapping and p2p propagation for high-performance data access");

        Ok(())
    }

    /// Forward a transaction to the TPU
    pub fn forward_transaction(&self, transaction: &Transaction) -> PluginResult<Signature> {
        if !self.tpu_forwarding_enabled {
            return Err(GeyserPluginError::Custom(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "TPU forwarding is not enabled",
                ),
            )));
        }

        let rpc_client = match &self.rpc_client {
            Some(client) => client,
            None => {
                return Err(GeyserPluginError::Custom(Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "No RPC connection available",
                    ),
                )));
            }
        };

        // Check if enough time has passed since last TPU forward
        let now = Instant::now();
        let mut last_forward = self.last_tpu_forward.write().unwrap();
        
        if now.duration_since(*last_forward).as_millis() < TPU_FORWARD_INTERVAL_MS as u128 {
            return Err(GeyserPluginError::Custom(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("TPU forwarding rate limited, try again after {} ms", TPU_FORWARD_INTERVAL_MS),
                ),
            )));
        }
        
        // Forward transaction to TPU via RPC
        let mut retries = 0;
        let signature = loop {
            match rpc_client.send_transaction(transaction) {
                Ok(signature) => break signature,
                Err(err) => {
                    retries += 1;
                    if retries >= MAX_RPC_RETRIES {
                        return Err(GeyserPluginError::Custom(Box::new(err)));
                    }
                    tracing::warn!("TPU forward failed (retry {}/{}): {}", retries, MAX_RPC_RETRIES, err);
                    std::thread::sleep(Duration::from_millis(100 * retries));
                }
            }
        };

        // Update last forward time
        *last_forward = now;
        
        tracing::debug!("Transaction forwarded to TPU: {}", signature);
        Ok(signature)
    }

    /// Get account data directly from validator
    pub fn get_account_data(&self, pubkey: &Pubkey) -> PluginResult<Option<Vec<u8>>> {
        let rpc_client = match &self.rpc_client {
            Some(client) => client,
            None => {
                return Err(GeyserPluginError::Custom(Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "No RPC connection available",
                    ),
                )));
            }
        };

        // Attempt to get account info with retries
        let mut retries = 0;
        loop {
            match rpc_client.get_account(pubkey) {
                Ok(account) => {
                    return Ok(Some(account.data));
                }
                Err(err) => {
                    // If account not found, return None
                    if err.to_string().contains("not found") {
                        return Ok(None);
                    }
                    
                    retries += 1;
                    if retries >= MAX_RPC_RETRIES {
                        return Err(GeyserPluginError::Custom(Box::new(err)));
                    }
                    tracing::warn!("Get account failed (retry {}/{}): {}", retries, MAX_RPC_RETRIES, err);
                    std::thread::sleep(Duration::from_millis(100 * retries));
                }
            }
        }
    }

    /// Update leader schedule
    pub fn update_leader_schedule(&self) -> PluginResult<()> {
        let rpc_client = match &self.rpc_client {
            Some(client) => client,
            None => {
                return Err(GeyserPluginError::Custom(Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "No RPC connection available",
                    ),
                )));
            }
        };

        // Get current slot
        let current_slot = match rpc_client.get_slot() {
            Ok(slot) => slot,
            Err(err) => {
                return Err(GeyserPluginError::Custom(Box::new(err)));
            }
        };

        // Get leader schedule for current epoch
        let leaders = match rpc_client.get_slot_leaders(current_slot, 100) {
            Ok(leaders) => leaders,
            Err(err) => {
                return Err(GeyserPluginError::Custom(Box::new(err)));
            }
        };

        // Update known validators
        {
            let mut known_validators = self.known_validators.write().unwrap();
            *known_validators = leaders;
        }

        // Get current leader
        if let Some(leader) = self.get_leader_for_slot(current_slot) {
            let mut current_leader = self.current_leader.write().unwrap();
            let mut current_leader_slot = self.current_leader_slot.write().unwrap();
            
            *current_leader = Some(leader);
            *current_leader_slot = Some(current_slot);
            
            tracing::debug!("Updated leader for slot {}: {}", current_slot, leader);
        }

        Ok(())
    }

    /// Get leader for a specific slot
    pub fn get_leader_for_slot(&self, slot: u64) -> Option<Pubkey> {
        // If we have RPC, try to get the leader directly
        if let Some(rpc_client) = &self.rpc_client {
            match rpc_client.get_slot_leader() {
                Ok(leader) => return Some(leader),
                Err(err) => {
                    tracing::warn!("Failed to get slot leader via RPC: {}", err);
                }
            }
        }
        
        // Fall back to cached leader schedule
        let known_validators = self.known_validators.read().unwrap();
        if known_validators.is_empty() {
            return None;
        }
        
        // Simple modulo-based lookup for leader in cached schedule
        let index = (slot as usize) % known_validators.len();
        Some(known_validators[index])
    }

    /// Check if the validator is a leader for the current slot
    pub fn is_leader(&self) -> bool {
        // Check if we have current leader information
        let current_leader = self.current_leader.read().unwrap();
        if current_leader.is_none() {
            return false;
        }
        
        // Ideally, we'd compare with validator identity, but that's not available
        // through the plugin interface. For now, we'll just return a placeholder.
        tracing::debug!("Leader check requested, returning false as this requires validator identity");
        false
    }

    /// Get the current slot
    pub fn get_current_slot(&self) -> PluginResult<u64> {
        let rpc_client = match &self.rpc_client {
            Some(client) => client,
            None => {
                return Err(GeyserPluginError::Custom(Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "No RPC connection available",
                    ),
                )));
            }
        };

        match rpc_client.get_slot() {
            Ok(slot) => Ok(slot),
            Err(err) => Err(GeyserPluginError::Custom(Box::new(err))),
        }
    }

    /// Check if RPC connection is available
    pub fn has_rpc_connection(&self) -> bool {
        self.rpc_client.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::hash::Hash;
    use solana_sdk::message::Message;
    use solana_sdk::signature::{Keypair, Signer};
    use solana_sdk::system_instruction;

    #[test]
    fn test_validator_integration_creation() {
        let integration = ValidatorIntegration::new(None).unwrap();
        assert!(integration.rpc_client.is_none());
        assert!(!integration.tpu_forwarding_enabled);
    }

    #[test]
    fn test_enable_tpu_forwarding_no_rpc() {
        let mut integration = ValidatorIntegration::new(None).unwrap();
        let result = integration.enable_tpu_forwarding();
        assert!(result.is_err());
    }

    #[test]
    fn test_tpu_tap_points() {
        let integration = ValidatorIntegration::new(None).unwrap();
        
        // This should not fail, just log warnings
        let result = integration.connect_tpu_tap(TpuTapPoint::Receive);
        assert!(result.is_ok());
        
        let result = integration.connect_tpu_tap(TpuTapPoint::Forward);
        assert!(result.is_ok());
        
        let result = integration.connect_tpu_tap(TpuTapPoint::Vote);
        assert!(result.is_ok());
        
        let result = integration.connect_tpu_tap(TpuTapPoint::BankCommit);
        assert!(result.is_ok());
    }

    #[test]
    fn test_transaction_forward_disabled() {
        let integration = ValidatorIntegration::new(None).unwrap();
        
        // Create a simple transaction
        let payer = Keypair::new();
        let to_pubkey = Pubkey::new_unique();
        let lamports = 100;
        let blockhash = Hash::default();
        
        let instruction = system_instruction::transfer(&payer.pubkey(), &to_pubkey, lamports);
        let message = Message::new(&[instruction], Some(&payer.pubkey()));
        let transaction = Transaction::new(&[&payer], message, blockhash);
        
        // Attempt to forward (should fail because forwarding is disabled)
        let result = integration.forward_transaction(&transaction);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_leader_for_slot_empty_schedule() {
        let integration = ValidatorIntegration::new(None).unwrap();
        let result = integration.get_leader_for_slot(42);
        assert!(result.is_none());
    }

    #[test]
    fn test_is_leader() {
        let integration = ValidatorIntegration::new(None).unwrap();
        assert!(!integration.is_leader());
    }
}

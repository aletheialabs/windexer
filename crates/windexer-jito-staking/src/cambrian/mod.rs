//! Cambrian integration for wIndexer Jito staking
//! 
//! This module implements the Cambrian Actively Validated Services (AVS)
//! and Proof-of-Authority (PoA) functionality for wIndexer.

use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::sync::RwLock;
use anyhow::Result;
use tracing::{info, warn, error};

mod avs;
mod poa;
mod payload;
mod operator;
mod oracle;

pub use avs::AvsManager;
pub use operator::OperatorManager;
pub use poa::{PoAState, ProposalInstructionData};
pub use payload::PayloadManager;
pub use oracle::OracleManager;

/// Configuration for Cambrian integration
#[derive(Debug, Clone)]
pub struct CambrianConfig {
    /// AVS IP address to bind to
    pub avs_ip: String,
    /// AVS HTTP port to bind to
    pub avs_http_port: u16,
    /// AVS WebSocket port to bind to
    pub avs_ws_port: u16,
    /// Admin keypair path
    pub admin_keypair_path: PathBuf,
    /// Solana API URL
    pub solana_api_url: String,
    /// Solana API WebSocket URL
    pub solana_ws_url: String,
    /// Cambrian Consensus Program name
    pub ccp_name: String,
    /// Proposal storage key
    pub proposal_storage_key: String,
    /// Storage space in bytes
    pub storage_space: u64,
    /// Consensus threshold
    pub consensus_threshold: f64,
    /// Stake threshold
    pub stake_threshold: u64,
}

impl Default for CambrianConfig {
    fn default() -> Self {
        Self {
            avs_ip: "127.0.0.1".to_string(),
            avs_http_port: 8080,
            avs_ws_port: 8081,
            admin_keypair_path: PathBuf::from("./admin-keypair.json"),
            solana_api_url: "http://localhost:8899".to_string(),
            solana_ws_url: "ws://localhost:8900".to_string(),
            ccp_name: format!("windexer-avs-{}", chrono::Utc::now().timestamp()),
            proposal_storage_key: format!("windexer-storage-{}", chrono::Utc::now().timestamp()),
            storage_space: 1024 * 1024, // 1 MB
            consensus_threshold: 0.66,
            stake_threshold: 1_000_000_000, // 1 SOL
        }
    }
}

/// Main service for Cambrian integration
pub struct CambrianService {
    config: CambrianConfig,
    avs_manager: Arc<AvsManager>,
    operator_manager: Arc<RwLock<OperatorManager>>,
    payload_manager: Arc<PayloadManager>,
    oracle_manager: Arc<OracleManager>,
    poa_state: Arc<RwLock<Option<PoAState>>>,
}

impl CambrianService {
    /// Create a new Cambrian service
    pub fn new(config: CambrianConfig) -> Self {
        let poa_state = Arc::new(RwLock::new(None));
        let avs_manager = Arc::new(AvsManager::new(config.clone()));
        let operator_manager = Arc::new(RwLock::new(OperatorManager::new(config.clone())));
        let payload_manager = Arc::new(PayloadManager::new(config.clone()));
        let oracle_manager = Arc::new(OracleManager::new(config.clone()));

        Self {
            config,
            avs_manager,
            operator_manager,
            payload_manager,
            oracle_manager,
            poa_state,
        }
    }

    /// Initialize AVS on-chain
    pub async fn initialize_avs(&self) -> Result<Pubkey> {
        info!("Initializing AVS on-chain");
        let poa_pubkey = self.avs_manager.initialize_avs().await?;
        
        info!("AVS initialized with PoA pubkey: {}", poa_pubkey);
        
        // Store the PoA state
        let poa_state = PoAState {
            pubkey: poa_pubkey,
            admin: Pubkey::new_unique(), // This would be the admin pubkey
            threshold: 2,                // Minimum operators to approve a proposal
            ncn: Pubkey::new_unique(),   // Network Coordination Network pubkey
            stake_threshold: self.config.stake_threshold,
        };
        
        *self.poa_state.write().await = Some(poa_state);
        
        Ok(poa_pubkey)
    }

    /// Start AVS server
    pub async fn start_avs(&self) -> Result<()> {
        info!("Starting AVS server");
        self.avs_manager.start_avs().await?;
        info!("AVS server started");
        Ok(())
    }

    /// Register a new operator
    pub async fn register_operator(&self, operator_pubkey: &Pubkey, stake: u64) -> Result<()> {
        info!("Registering operator: {}", operator_pubkey);
        let mut operator_manager = self.operator_manager.write().await;
        operator_manager.register_operator(operator_pubkey, stake).await?;
        info!("Operator registered successfully");
        Ok(())
    }

    /// Execute a proposal
    pub async fn execute_proposal(&self, payload_image: &str) -> Result<Signature> {
        info!("Executing proposal with payload: {}", payload_image);
        
        // Get the PoA state
        let poa_state = self.poa_state.read().await.clone().ok_or_else(|| {
            anyhow::anyhow!("PoA state not initialized")
        })?;
        
        // Run the payload
        let proposal_file = self.payload_manager.run_payload(payload_image, &poa_state).await?;
        
        // Submit the proposal to PoA program
        let signature = self.avs_manager.submit_proposal(&proposal_file, &poa_state).await?;
        
        info!("Proposal executed with signature: {}", signature);
        Ok(signature)
    }
}

// Helper to read a keypair from file
fn read_keypair(path: &PathBuf) -> Result<Keypair> {
    let keypair_bytes = std::fs::read(path)?;
    let keypair_str = String::from_utf8(keypair_bytes)?;
    let keypair_json: Vec<u8> = serde_json::from_str(&keypair_str)?;
    Ok(Keypair::from_bytes(&keypair_json)?)
} 
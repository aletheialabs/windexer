//! Operator manager for Cambrian integration

use super::CambrianConfig;
use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use tracing::info;

/// Operator information
#[derive(Debug, Clone)]
pub struct OperatorInfo {
    /// Operator public key
    pub pubkey: Pubkey,
    /// Staked amount
    pub stake: u64,
    /// Is operator active
    pub is_active: bool,
    /// Last seen timestamp
    pub last_seen: i64,
}

/// Operator manager
pub struct OperatorManager {
    config: CambrianConfig,
    operators: HashMap<Pubkey, OperatorInfo>,
}

impl OperatorManager {
    /// Create a new operator manager
    pub fn new(config: CambrianConfig) -> Self {
        Self {
            config,
            operators: HashMap::new(),
        }
    }
    
    /// Register a new operator
    pub async fn register_operator(&mut self, operator_pubkey: &Pubkey, stake: u64) -> Result<()> {
        info!("Registering operator: {}", operator_pubkey);
        
        // Check if stake is sufficient
        if stake < self.config.stake_threshold {
            return Err(anyhow::anyhow!(
                "Stake is below threshold: {} < {}",
                stake,
                self.config.stake_threshold
            ));
        }
        
        // Register operator
        let now = chrono::Utc::now().timestamp();
        
        let operator_info = OperatorInfo {
            pubkey: *operator_pubkey,
            stake,
            is_active: true,
            last_seen: now,
        };
        
        self.operators.insert(*operator_pubkey, operator_info);
        
        info!("Operator registered successfully");
        Ok(())
    }
    
    /// Get all active operators
    pub fn get_active_operators(&self) -> Vec<&OperatorInfo> {
        self.operators.values()
            .filter(|op| op.is_active)
            .collect()
    }
    
    /// Get operator info
    pub fn get_operator_info(&self, operator_pubkey: &Pubkey) -> Option<&OperatorInfo> {
        self.operators.get(operator_pubkey)
    }
    
    /// Update operator status
    pub fn update_operator_status(&mut self, operator_pubkey: &Pubkey, is_active: bool) -> Result<()> {
        let operator = self.operators.get_mut(operator_pubkey)
            .ok_or_else(|| anyhow::anyhow!("Operator not found: {}", operator_pubkey))?;
        
        operator.is_active = is_active;
        operator.last_seen = chrono::Utc::now().timestamp();
        
        Ok(())
    }
} 
// crates/windexer-jito-staking/src/staking/mod.rs

use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use tracing::info;
use std::time::Duration;

pub mod types;
mod vault;
mod delegation;

use types::*;
use vault::VaultManager;
use delegation::DelegationManager;

#[derive(Debug)]
pub struct StakingManager {
    vault_manager: Arc<RwLock<VaultManager>>,
    delegation_manager: Arc<RwLock<DelegationManager>>,
    config: StakingConfig,
}

impl StakingManager {
    pub fn new(config: StakingConfig) -> Self {
        Self {
            vault_manager: Arc::new(RwLock::new(VaultManager::new())),
            delegation_manager: Arc::new(RwLock::new(DelegationManager::new())),
            config,
        }
    }

    pub async fn process_stake(&self, amount: u64, staker: Pubkey, operator: Pubkey) -> Result<()> {
        if amount < self.config.min_stake {
            return Err(anyhow::anyhow!("Stake amount below minimum threshold"));
        }

        let mut vault_mgr = self.vault_manager.write().await;
        let mut delegation_mgr = self.delegation_manager.write().await;

        delegation_mgr.record_stake(staker, operator, amount).await?;
        vault_mgr.increase_stake(operator, amount).await?;

        info!("Processed stake of {} lamports from {} to operator {}", 
              amount, staker, operator);
        Ok(())
    }

    pub async fn get_operator_stats(&self, operator: &Pubkey) -> Result<OperatorStats> {
        let vault_mgr = self.vault_manager.read().await;
        let delegation_mgr = self.delegation_manager.read().await;

        let total_stake = vault_mgr.get_total_stake(operator).await?;
        let delegations = delegation_mgr.get_operator_delegations(operator).await?;
        
        Ok(OperatorStats {
            total_stake,
            delegation_count: delegations.len() as u64,
            commission_bps: self.config.commission_bps,
        })
    }
}

// Types and helper structs
#[derive(Debug, Clone)]
pub struct StakingConfig {
    pub min_stake: u64,
    pub commission_bps: u16,
    pub min_delegation_period: Duration,
    pub max_operator_stake: u64,
}

#[derive(Debug)]
pub struct OperatorStats {
    pub total_stake: u64,
    pub delegation_count: u64, 
    pub commission_bps: u16,
}
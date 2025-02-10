// crates/windexer-jito-staking/src/staking/mod.rs

use solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

mod delegation;
pub mod types;
mod vault;

pub use types::{StakingConfig, OperatorStats};
pub use delegation::DelegationManager;
pub use vault::VaultManager;

pub struct StakingManager {
    config: StakingConfig,
    delegation_manager: RwLock<DelegationManager>,
    vault_manager: RwLock<VaultManager>,
}

impl StakingManager {
    pub fn new(config: StakingConfig) -> Self {
        Self {
            config,
            delegation_manager: RwLock::new(DelegationManager::new()),
            vault_manager: RwLock::new(VaultManager::new()),
        }
    }

    pub fn config(&self) -> &StakingConfig {
        &self.config
    }

    pub async fn process_stake(
        &self,
        amount: u64,
        staker: Pubkey,
        operator: Pubkey,
    ) -> Result<()> {
        if amount < self.config.min_stake {
            return Err(anyhow::anyhow!("Stake amount below minimum threshold"));
        }

        let stats = self.get_operator_stats(&operator).await?;
        if stats.total_stake + amount > self.config.max_operator_stake {
            return Err(anyhow::anyhow!("Operator would exceed maximum stake"));
        }

        let mut delegation_manager = self.delegation_manager.write().await;
        delegation_manager.add_delegation(operator, staker, amount).await?;

        Ok(())
    }

    pub async fn get_operator_stats(&self, operator: &Pubkey) -> Result<OperatorStats> {
        let delegation_manager = self.delegation_manager.read().await;
        let delegations = delegation_manager.get_operator_delegations(operator);
        let total_stake: u64 = delegations.iter().map(|(_, amount)| amount).sum();
        
        Ok(OperatorStats {
            total_stake,
            active_delegations: delegations.len() as u32,
            commission_earned: 0,
            uptime: 1.0,
            last_active: chrono::Utc::now().timestamp(),
        })
    }

    pub async fn create_vault(
        &self,
        admin: Pubkey,
        mint: Pubkey,
        ncn: Pubkey
    ) -> Result<Pubkey> {
        let mut vault_manager = self.vault_manager.write().await;
        vault_manager.create_vault(admin, mint, ncn).await
    }

    pub async fn add_delegation_to_vault(
        &self,
        vault: Pubkey,
        operator: Pubkey,
        amount: u64
    ) -> Result<()> {
        let stats = self.get_operator_stats(&operator).await?;
        if stats.total_stake < amount {
            return Err(anyhow::anyhow!("Insufficient stake for vault delegation"));
        }

        let vault_manager = self.vault_manager.read().await;
        vault_manager.add_delegation(vault, operator, amount).await
    }
}
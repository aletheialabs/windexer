// crates/windexer-jito-staking/src/staking/mod.rs

//! Staking management module

pub mod types;

use {
    std::{collections::HashMap, sync::RwLock},
    solana_sdk::pubkey::Pubkey,
    anyhow::Result,
    crate::staking::types::{StakingConfig, OperatorStats},
};

mod delegation;
mod vault;

pub use delegation::DelegationManager;
pub use vault::VaultManager;

pub struct StakingManager {
    config: StakingConfig,
    operators: RwLock<HashMap<Pubkey, OperatorStats>>,
}

impl StakingManager {
    pub fn new(config: StakingConfig) -> Self {
        Self {
            config,
            operators: RwLock::new(HashMap::new()),
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
        if stats.total_stake + amount > 1_000_000_000_000 {
            return Err(anyhow::anyhow!("Operator would exceed maximum stake"));
        }

        let mut operators = self.operators.write().unwrap();
        let stats = operators.entry(operator).or_default();
        stats.total_stake += amount;

        Ok(())
    }

    pub async fn get_operator_stats(&self, operator: &Pubkey) -> Result<OperatorStats> {
        let operators = self.operators.read().unwrap();
        let stats = operators.get(operator).cloned().unwrap_or_default();
        Ok(stats)
    }
}
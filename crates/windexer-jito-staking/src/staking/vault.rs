// crates/windexer-jito-staking/src/staking/vault.rs

use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use anyhow::Result;
use tracing::info;
use chrono;

#[derive(Debug)]
pub struct VaultManager {
    vaults: HashMap<Pubkey, VaultState>,
    config: VaultConfig,
}

#[derive(Debug)]
pub struct VaultState {
    pub total_staked: u64,
    pub operators: HashMap<Pubkey, OperatorState>,
    pub last_update: i64,
}

#[derive(Debug)]
pub struct OperatorState {
    pub stake_amount: u64,
    pub reward_balance: u64,
    pub performance_score: f64,
}

#[derive(Debug, Clone)]
pub struct VaultConfig {
    pub max_operator_stake: u64,
    pub min_operator_stake: u64,
    pub performance_threshold: f64,
}

impl VaultManager {
    pub fn new() -> Self {
        Self {
            vaults: HashMap::new(),
            config: VaultConfig {
                max_operator_stake: 1_000_000_000_000,
                min_operator_stake: 1_000_000_000,
                performance_threshold: 0.95,
            },
        }
    }

    pub async fn increase_stake(&mut self, operator: Pubkey, amount: u64) -> Result<()> {
        let max_stake = self.config.max_operator_stake;
        let operator_state = self.get_or_create_operator_state(&operator)?;
        
        if operator_state.stake_amount + amount > max_stake {
            return Err(anyhow::anyhow!("Exceeds maximum operator stake"));
        }

        operator_state.stake_amount += amount;
        info!("Increased stake for operator {} by {} lamports", operator, amount);
        Ok(())
    }

    pub async fn decrease_stake(&mut self, operator: Pubkey, amount: u64) -> Result<()> {
        let min_stake = self.config.min_operator_stake;
        let operator_state = self.get_operator_state_mut(&operator)?;
        
        if operator_state.stake_amount < amount {
            return Err(anyhow::anyhow!("Insufficient stake balance"));
        }

        if operator_state.stake_amount - amount < min_stake {
            return Err(anyhow::anyhow!("Would fall below minimum stake threshold"));
        }

        operator_state.stake_amount -= amount;
        info!("Decreased stake for operator {} by {} lamports", operator, amount);
        Ok(())
    }

    pub async fn get_total_stake(&self, operator: &Pubkey) -> Result<u64> {
        let operator_state = self.get_operator_state(operator)?;
        Ok(operator_state.stake_amount)
    }

    fn get_operator_state(&self, operator: &Pubkey) -> Result<&OperatorState> {
        self.vaults
            .values()
            .find_map(|v| v.operators.get(operator))
            .ok_or_else(|| anyhow::anyhow!("Operator not found"))
    }

    fn get_operator_state_mut(&mut self, operator: &Pubkey) -> Result<&mut OperatorState> {
        self.vaults
            .values_mut()
            .find_map(|v| v.operators.get_mut(operator))
            .ok_or_else(|| anyhow::anyhow!("Operator not found"))
    }

    fn get_or_create_operator_state(&mut self, operator: &Pubkey) -> Result<&mut OperatorState> {
        let vault = self.vaults.entry(Pubkey::new_unique()).or_insert(VaultState {
            total_staked: 0,
            operators: HashMap::new(),
            last_update: chrono::Utc::now().timestamp(),
        });

        Ok(vault.operators.entry(*operator).or_insert(OperatorState {
            stake_amount: 0,
            reward_balance: 0,
            performance_score: 1.0,
        }))
    }
}
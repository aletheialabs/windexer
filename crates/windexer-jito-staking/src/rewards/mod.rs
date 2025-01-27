//! Reward distribution and calculation for staking participants

use crate::{Result, StakingConfig};
use solana_program::pubkey::Pubkey;
use std::collections::HashMap;

/// Manages reward distribution for staking participants
pub struct RewardDistributor {
    /// Reward calculation configuration
    config: StakingConfig,
    /// Accumulated rewards by validator
    rewards: HashMap<Pubkey, u64>,
    /// Last distribution timestamp
    last_distribution: std::time::SystemTime,
}

impl RewardDistributor {
    /// Creates a new reward distributor
    pub fn new(config: StakingConfig) -> Self {
        Self {
            config,
            rewards: HashMap::new(),
            last_distribution: std::time::SystemTime::now(),
        }
    }

    /// Calculates rewards for a validator based on stake and participation
    pub async fn calculate_rewards(&self, validator: &Pubkey, stake: u64) -> Result<u64> {
        let base_reward = (stake as f64 * self.config.reward_rate) as u64;
        
        // Apply multipliers based on participation and performance
        let participation_multiplier = self.get_participation_multiplier(validator).await?;
        
        Ok((base_reward as f64 * participation_multiplier) as u64)
    }

    /// Distributes accumulated rewards to validators
    pub async fn distribute_rewards(&mut self) -> Result<()> {
        let now = std::time::SystemTime::now();
        
        // Only distribute if reward window has elapsed
        if now.duration_since(self.last_distribution)? < self.config.reward_window {
            return Ok(());
        }

        for (validator, rewards) in self.rewards.iter() {
            // Transfer rewards to validator
            self.transfer_rewards(validator, *rewards).await?;
        }

        // Reset rewards after distribution
        self.rewards.clear();
        self.last_distribution = now;

        Ok(())
    }

    async fn get_participation_multiplier(&self, validator: &Pubkey) -> Result<f64> {
        // Calculate multiplier based on validator's participation
        // This would integrate with Jito's participation tracking
        Ok(1.0) // Default multiplier
    }

    async fn transfer_rewards(&self, validator: &Pubkey, amount: u64) -> Result<()> {
        // Integrate with Solana program for reward transfer
        Ok(())
    }
}
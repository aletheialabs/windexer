// crates/windexer-jito-staking/src/rewards/mod.rs

use solana_sdk::pubkey::Pubkey;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use anyhow::Result;

pub mod calculation;
pub mod distribution;

pub struct RewardsManager {
    reward_calculator: Arc<RwLock<calculation::RewardCalculator>>,
    reward_distributor: Arc<RwLock<distribution::RewardDistributor>>,
    epoch_rewards: Arc<RwLock<HashMap<Pubkey, u64>>>,
    reward_rate: f64,
    distribution_interval: Duration,
}

impl RewardsManager {
    pub fn new(reward_rate: f64, distribution_interval: Duration) -> Self {
        Self {
            reward_calculator: Arc::new(RwLock::new(calculation::RewardCalculator::new(reward_rate))),
            reward_distributor: Arc::new(RwLock::new(distribution::RewardDistributor::new(distribution_interval))),
            epoch_rewards: Arc::new(RwLock::new(HashMap::new())),
            reward_rate,
            distribution_interval,
        }
    }

    pub async fn calculate_operator_rewards(&self, operator: &Pubkey, performance_score: f64) -> Result<u64> {
        let calculator = self.reward_calculator.read().await;
        let mut rewards = self.epoch_rewards.write().await;
        
        let reward_amount = calculator.calculate_reward(operator, performance_score).await?;
        *rewards.entry(*operator).or_default() += reward_amount;
        
        Ok(reward_amount)
    }

    pub async fn distribute_rewards(&self) -> Result<()> {
        let distributor = self.reward_distributor.read().await;
        let rewards = self.epoch_rewards.read().await;
        
        distributor.distribute_epoch_rewards(&rewards).await?;
        Ok(())
    }

    pub async fn distribution_interval(&self) -> Duration {
        self.distribution_interval
    }
}

impl Default for RewardsManager {
    fn default() -> Self {
        Self::new(0.1, Duration::from_secs(86400)) // Default values
    }
}
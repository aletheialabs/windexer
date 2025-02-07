// crates/windexer-jito-staking/src/rewards/calculation.rs

use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use anyhow::Result;

pub struct RewardCalculator {
    base_reward_rate: f64,
    performance_multipliers: HashMap<Pubkey, f64>,
}

impl RewardCalculator {
    pub fn new(base_rate: f64) -> Self {
        Self {
            base_reward_rate: base_rate,
            performance_multipliers: HashMap::new(),
        }
    }

    pub async fn calculate_reward(&self, operator: &Pubkey, performance_score: f64) -> Result<u64> {
        let multiplier = self.performance_multipliers
            .get(operator)
            .copied()
            .unwrap_or(1.0);
            
        let reward = (self.base_reward_rate * performance_score * multiplier) as u64;
        Ok(reward)
    }

    pub fn update_performance_multiplier(&mut self, operator: &Pubkey, multiplier: f64) {
        self.performance_multipliers.insert(*operator, multiplier);
    }
}
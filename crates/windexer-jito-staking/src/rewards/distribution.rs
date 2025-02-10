// crates/windexer-jito-staking/src/rewards/distribution.rs

use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use anyhow::Result;
use tokio::time::Duration;
use tracing::{info, warn};
use std::sync::RwLock;

pub struct RewardDistributor {
    distribution_interval: Duration,
    last_distribution: RwLock<i64>,
    pending_distributions: RwLock<HashMap<Pubkey, u64>>,
}

impl RewardDistributor {
    pub fn new(interval: Duration) -> Self {
        Self {
            distribution_interval: interval,
            last_distribution: RwLock::new(0),
            pending_distributions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn distribute_epoch_rewards(&self, rewards: &HashMap<Pubkey, u64>) -> Result<()> {
        let current_time = chrono::Utc::now().timestamp();
        
        {
            let last_dist = *self.last_distribution.read().unwrap();
            if current_time - last_dist < self.distribution_interval.as_secs() as i64 {
                return Ok(());
            }
        }

        {
            *self.last_distribution.write().unwrap() = current_time;
        }

        for (operator, amount) in rewards {
            match self.execute_distribution(operator, *amount).await {
                Ok(_) => {
                    info!("Distributed {} rewards to operator {}", amount, operator);
                }
                Err(e) => {
                    warn!("Failed to distribute rewards to {}: {}", operator, e);
                    self.pending_distributions.write().unwrap().insert(*operator, *amount);
                }
            }
        }

        Ok(())
    }

    async fn execute_distribution(&self, operator: &Pubkey, amount: u64) -> Result<()> {
        // Handle pending distributions first
        let pending = {
            let mut pending_dist = self.pending_distributions.write().unwrap();
            pending_dist.remove(operator)
        };

        if let Some(pending_amount) = pending {
            self.process_distribution(operator, pending_amount).await?;
        }

        // Process current distribution
        self.process_distribution(operator, amount).await?;

        Ok(())
    }

    async fn process_distribution(&self, operator: &Pubkey, amount: u64) -> Result<()> {
        let commission_rate = 0.10;
        let commission = (amount as f64 * commission_rate) as u64;
        let net_amount = amount - commission;

        self.transfer_commission(operator, commission).await?;

        self.distribute_to_delegators(operator, net_amount).await?;

        info!(
            "Processed distribution for operator {}: amount={}, commission={}, net={}",
            operator, amount, commission, net_amount
        );

        Ok(())
    }

    async fn transfer_commission(&self, operator: &Pubkey, amount: u64) -> Result<()> {
        info!("Transferring commission {} to operator {}", amount, operator);
        Ok(())
    }

    async fn distribute_to_delegators(&self, operator: &Pubkey, amount: u64) -> Result<()> {
        info!("Distributing {} to delegators of operator {}", amount, operator);
        Ok(())
    }

    pub fn distribution_interval(&self) -> Duration {
        self.distribution_interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reward_distribution() {
        let distributor = RewardDistributor::new(Duration::from_secs(3600));
        let mut rewards = HashMap::new();
        rewards.insert(Pubkey::new_unique(), 1000);
        
        assert!(distributor.distribute_epoch_rewards(&rewards).await.is_ok());
    }

    #[tokio::test]
    async fn test_pending_distributions() {
        let distributor = RewardDistributor::new(Duration::from_secs(3600));
        let operator = Pubkey::new_unique();
        
        distributor.pending_distributions.write().unwrap().insert(operator, 500);
        let amount = 1000;
        
        assert!(distributor.execute_distribution(&operator, amount).await.is_ok());
        assert!(distributor.pending_distributions.read().unwrap().is_empty());
    }
}
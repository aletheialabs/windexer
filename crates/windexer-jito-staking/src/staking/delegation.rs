// crates/windexer-jito-staking/src/staking/delegation.rs

use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use anyhow::Result;
use tracing::{info, warn};

#[derive(Debug)]
pub struct DelegationManager {
    delegations: HashMap<Pubkey, StakerInfo>,
    operator_delegations: HashMap<Pubkey, Vec<Pubkey>>,
}

#[derive(Debug)]
pub struct StakerInfo {
    pub total_stake: u64,
    pub delegations: HashMap<Pubkey, DelegationInfo>,
    pub last_stake: i64,
}

#[derive(Debug, Clone)]
pub struct DelegationInfo {
    pub amount: u64,
    pub operator: Pubkey,
    pub start_time: i64,
    pub rewards_claimed: u64,
}

impl DelegationManager {
    pub fn new() -> Self {
        Self {
            delegations: HashMap::new(),
            operator_delegations: HashMap::new(),
        }
    }

    pub async fn record_stake(
        &mut self,
        staker: Pubkey,
        operator: Pubkey,
        amount: u64,
    ) -> Result<()> {
        let staker_info = self.delegations.entry(staker).or_insert(StakerInfo {
            total_stake: 0,
            delegations: HashMap::new(),
            last_stake: chrono::Utc::now().timestamp(),
        });

        let delegation = staker_info.delegations.entry(operator).or_insert(DelegationInfo {
            amount: 0,
            operator,
            start_time: chrono::Utc::now().timestamp(),
            rewards_claimed: 0,
        });

        delegation.amount += amount;
        staker_info.total_stake += amount;

        // Track operator delegations
        self.operator_delegations
            .entry(operator)
            .or_default()
            .push(staker);

        info!("Recorded stake: {} lamports from {} to operator {}", 
              amount, staker, operator);
        Ok(())
    }

    pub async fn get_operator_delegations(&self, operator: &Pubkey) -> Result<Vec<DelegationInfo>> {
        let stakers = self.operator_delegations
            .get(operator)
            .ok_or_else(|| anyhow::anyhow!("No delegations found for operator"))?;

        let mut delegations = Vec::new();
        for staker in stakers {
            if let Some(staker_info) = self.delegations.get(staker) {
                if let Some(delegation) = staker_info.delegations.get(operator) {
                    delegations.push(delegation.clone());
                }
            }
        }

        Ok(delegations)
    }

    pub async fn claim_rewards(
        &mut self,
        staker: &Pubkey,
        operator: &Pubkey,
        amount: u64,
    ) -> Result<()> {
        let staker_info = self.delegations
            .get_mut(staker)
            .ok_or_else(|| anyhow::anyhow!("Staker not found"))?;

        let delegation = staker_info.delegations
            .get_mut(operator)
            .ok_or_else(|| anyhow::anyhow!("Delegation not found"))?;

        delegation.rewards_claimed += amount;
        info!("Claimed {} lamports in rewards for staker {} from operator {}", 
              amount, staker, operator);
        Ok(())
    }
}
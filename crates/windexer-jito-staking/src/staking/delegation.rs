// crates/windexer-jito-staking/src/staking/delegation.rs

use solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use std::collections::HashMap;

pub struct DelegationManager {
    delegations: HashMap<Pubkey, Vec<(Pubkey, u64)>>, // operator -> [(staker, amount)]
}

impl DelegationManager {
    pub fn new() -> Self {
        Self {
            delegations: HashMap::new()
        }
    }

    pub async fn add_delegation(
        &mut self,
        operator: Pubkey,
        staker: Pubkey,
        amount: u64
    ) -> Result<()> {
        let operator_delegations = self.delegations
            .entry(operator)
            .or_insert_with(Vec::new);
            
        // Check for existing delegation from this staker
        if let Some(pos) = operator_delegations.iter().position(|(s, _)| s == &staker) {
            operator_delegations[pos].1 += amount;
        } else {
            operator_delegations.push((staker, amount));
        }
            
        Ok(())
    }

    pub fn get_operator_delegations(&self, operator: &Pubkey) -> Vec<(Pubkey, u64)> {
        self.delegations
            .get(operator)
            .cloned()
            .unwrap_or_default()
    }

    pub fn remove_delegation(
        &mut self,
        operator: &Pubkey,
        staker: &Pubkey
    ) -> Result<u64> {
        if let Some(delegations) = self.delegations.get_mut(operator) {
            if let Some(pos) = delegations.iter().position(|(s, _)| s == staker) {
                let (_, amount) = delegations.remove(pos);
                return Ok(amount);
            }
        }
        Err(anyhow::anyhow!("Delegation not found"))
    }
}
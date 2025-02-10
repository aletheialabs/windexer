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
        self.delegations
            .entry(operator)
            .or_insert_with(Vec::new)
            .push((staker, amount));
            
        Ok(())
    }

    pub fn get_operator_delegations(&self, operator: &Pubkey) -> Vec<(Pubkey, u64)> {
        self.delegations
            .get(operator)
            .cloned()
            .unwrap_or_default()
    }
}
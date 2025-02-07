// crates/windexer-jito-staking/src/slashing/penalties.rs

use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use anyhow::Result;
use crate::slashing::ViolationType;

pub struct PenaltyCalculator {
    base_penalties: HashMap<ViolationType, u64>,
    operator_multipliers: HashMap<Pubkey, f64>,
}

impl PenaltyCalculator {
    pub fn new() -> Self {
        let mut base_penalties = HashMap::new();
        base_penalties.insert(ViolationType::Downtime, 1000);
        base_penalties.insert(ViolationType::InvalidConsensus, 5000);
        base_penalties.insert(ViolationType::MaliciousBehavior, 10000);

        Self {
            base_penalties,
            operator_multipliers: HashMap::new(),
        }
    }

    pub async fn calculate_penalty(&self, operator: &Pubkey, violation: &ViolationType) -> Result<u64> {
        let base_penalty = self.base_penalties.get(violation)
            .ok_or_else(|| anyhow::anyhow!("Unknown violation type"))?;
            
        let multiplier = self.operator_multipliers
            .get(operator)
            .copied()
            .unwrap_or(1.0);
            
        Ok(((*base_penalty as f64) * multiplier) as u64)
    }

    pub fn update_operator_multiplier(&mut self, operator: &Pubkey, multiplier: f64) {
        self.operator_multipliers.insert(*operator, multiplier);
    }
}
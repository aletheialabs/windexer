// crates/windexer-jito-staking/src/slashing/penalties.rs

use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use anyhow::Result;
use crate::slashing::ViolationType;

pub struct PenaltyCalculator {
    base_penalties: HashMap<ViolationType, u64>,
}

impl PenaltyCalculator {
    pub fn new() -> Self {
        let mut base_penalties = HashMap::new();
        
        // Initialize base penalty amounts for each violation type
        base_penalties.insert(ViolationType::LowUptime, 1000);
        base_penalties.insert(ViolationType::DoubleProposal, 5000);
        base_penalties.insert(ViolationType::DoubleVote, 7500);
        base_penalties.insert(ViolationType::MaliciousValidation, 10000);
        
        Self {
            base_penalties,
        }
    }
    
    pub async fn calculate_penalty(&self, _operator: &Pubkey, violation: &ViolationType) -> Result<u64> {
        // Get base penalty for the violation type
        let base_penalty = self.base_penalties.get(violation)
            .copied()
            .unwrap_or(1000); // Default penalty if violation type not found
            
        // Here you could implement more complex logic to adjust penalty based on
        // operator's history, stake amount, etc.
        
        Ok(base_penalty)
    }
}
// crates/windexer-jito-staking/src/consensus/mod.rs

use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use anyhow::Result;

#[derive(Debug)]
pub struct ConsensusManager {
    operators: HashMap<Pubkey, OperatorInfo>,
    ncn_network: NCNNetwork,
}

#[derive(Debug)]
pub struct OperatorInfo {
    pub stake: u64,
    pub performance_score: f64,
    pub last_heartbeat: i64,
}

#[derive(Debug)]
pub struct NCNNetwork {
    pub min_operators: usize,
    pub total_stake: u64,
    pub consensus_threshold: f64,
}

impl ConsensusManager {
    pub fn new(min_operators: usize, consensus_threshold: f64) -> Self {
        Self {
            operators: HashMap::new(),
            ncn_network: NCNNetwork {
                min_operators,
                total_stake: 0,
                consensus_threshold,
            },
        }
    }

    pub async fn process_operator_heartbeat(
        &mut self,
        operator: Pubkey,
    ) -> Result<()> {
        let operator_info = self.operators.entry(operator)
            .or_insert(OperatorInfo {
                stake: 0,
                performance_score: 1.0,
                last_heartbeat: 0,
            });

        operator_info.last_heartbeat = chrono::Utc::now().timestamp();
        Ok(())
    }

    pub fn check_consensus_participation(&self) -> bool {
        let active_stake: u64 = self.operators
            .values()
            .filter(|op| op.performance_score >= 0.95)
            .map(|op| op.stake)
            .sum();

        let participation_rate = active_stake as f64 / self.ncn_network.total_stake as f64;
        participation_rate >= self.ncn_network.consensus_threshold
    }
}
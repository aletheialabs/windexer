// crates/windexer-jito-staking/src/staking/types.rs

use solana_sdk::pubkey::Pubkey;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StakingConfig {
    pub min_stake: u64,
    pub min_operators: u32,
    pub consensus_threshold: f64,
    pub reward_rate: f64,
    pub distribution_interval: Duration,
    pub slash_threshold: f64,
    pub min_uptime: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OperatorStats {
    pub pubkey: Option<Pubkey>,
    pub total_stake: u64,
    pub active_delegations: HashMap<Pubkey, u64>,
    pub last_active: Option<i64>,
    pub performance_score: f64,
}

#[derive(Debug)]
pub struct DelegationInfo {
    pub staker: Pubkey,
    pub operator: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}
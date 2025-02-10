// crates/windexer-jito-staking/src/staking/types.rs

use solana_sdk::pubkey::Pubkey;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct StakingConfig {
    pub min_stake: u64,
    pub commission_bps: u16,
    pub min_delegation_period: Duration,
    pub max_operator_stake: u64,
    pub min_operators: u32,
    pub consensus_threshold: f64,
    pub reward_rate: f64,
    pub distribution_interval: Duration,
    pub slash_threshold: f64,
    pub min_uptime: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorStats {
    pub total_stake: u64,
    pub active_delegations: u32,
    pub commission_earned: u64,
    pub uptime: f64,
    pub last_active: i64,
}

#[derive(Debug)]
pub struct DelegationInfo {
    pub staker: Pubkey,
    pub operator: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}
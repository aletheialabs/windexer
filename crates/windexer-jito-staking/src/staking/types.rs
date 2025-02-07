// crates/windexer-jito-staking/src/staking/types.rs

// use solana_sdk::pubkey::Pubkey;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct VaultConfig {
    pub max_operator_stake: u64,
    pub min_operator_stake: u64,
    pub performance_threshold: f64,
}

#[derive(Debug, Clone)]
pub struct StakingConfig {
    pub min_stake: u64,
    pub commission_bps: u16,
    pub min_delegation_period: Duration,
    pub max_operator_stake: u64,
}

#[derive(Debug)]
pub struct OperatorStats {
    pub total_stake: u64,
    pub delegation_count: u64,
    pub commission_bps: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViolationType {
    Downtime,
    InvalidConsensus,
    MaliciousBehavior,
}
//! Implementation of Jito staking integration for wIndexer
//! Provides staking-based validation and rewards distribution

pub mod consensus;
pub mod rewards;
pub mod slashing;
pub mod staking;

use solana_program::pubkey::Pubkey;
use thiserror::Error;

/// Errors that can occur in staking operations
#[derive(Error, Debug)]
pub enum StakingError {
    #[error("Invalid stake amount: {0}")]
    InvalidStakeAmount(String),

    #[error("Insufficient stake: {0}")]
    InsufficientStake(String),

    #[error("Slashing condition detected: {0}")]
    SlashingCondition(String),

    #[error("Reward distribution failed: {0}")]
    RewardDistributionError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, StakingError>;

/// Core staking configuration parameters
#[derive(Debug, Clone)]
pub struct StakingConfig {
    /// Minimum stake required for validation
    pub min_stake: u64,
    /// Reward rate per epoch
    pub reward_rate: f64,
    /// Slashing percentage for violations
    pub slash_percentage: f64,
    /// Time window for calculating rewards
    pub reward_window: std::time::Duration,
}
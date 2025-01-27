//! Consensus module integrating with Jito staking for validator consensus
use std::time::Duration;

pub mod protocol;
pub mod state;
pub mod validator;

#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    // Jito staking configuration
    pub stake_account: String,
    pub validator_identity: String,
    pub min_stake_amount: u64,
    
    // Network parameters
    pub block_time: Duration,
    pub sync_timeout: Duration,
    pub max_block_size: usize,
    pub min_validator_count: usize,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            stake_account: String::new(),
            validator_identity: String::new(),
            min_stake_amount: 1_000_000_000, // 1 SOL
            block_time: Duration::from_millis(400),
            sync_timeout: Duration::from_secs(10),
            max_block_size: 65536,
            min_validator_count: 4,
        }
    }
}
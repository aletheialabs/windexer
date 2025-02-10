// crates/windexer-jito-staking/src/consensus/mod.rs

use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub struct ConsensusManager {
    min_operators: usize,
    consensus_threshold: f64,
    active_operators: Arc<RwLock<Vec<Pubkey>>>,
}

impl ConsensusManager {
    pub fn new(min_operators: usize, consensus_threshold: f64) -> Self {
        Self {
            min_operators,
            consensus_threshold,
            active_operators: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register_operator(&self, operator: Pubkey) -> Result<()> {
        let mut operators = self.active_operators.write().await;
        if !operators.contains(&operator) {
            operators.push(operator);
            info!("Registered new operator: {}", operator);
        }
        Ok(())
    }

    pub async fn check_consensus_threshold(&self) -> Result<bool> {
        let operators = self.active_operators.read().await;
        if operators.len() < self.min_operators {
            warn!("Not enough operators for consensus");
            return Ok(false);
        }

        let active_ratio = operators.len() as f64 / self.min_operators as f64;
        Ok(active_ratio >= self.consensus_threshold)
    }
}
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct ConsensusMetrics {
    active_operators: AtomicU64,
    proposals_processed: AtomicU64,
    votes_processed: AtomicU64,
    consensus_rounds: AtomicU64,
}

impl ConsensusMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn update_active_operators(&self, count: u64) {
        self.active_operators.store(count, Ordering::Relaxed);
    }

    pub fn increment_proposals(&self) {
        self.proposals_processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_votes(&self) {
        self.votes_processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_consensus_rounds(&self) {
        self.consensus_rounds.fetch_add(1, Ordering::Relaxed);
    }
} 
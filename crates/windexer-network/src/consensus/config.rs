#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    pub min_validators: usize,
    pub consensus_threshold: f64,
    pub block_time: u64,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            min_validators: 4,
            consensus_threshold: 0.66,
            block_time: 400,
        }
    }
} 
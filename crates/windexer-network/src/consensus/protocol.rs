// crates/windexer-network/src/consensus/protocol.rs

use {
    std::sync::Arc,
    tokio::sync::{mpsc, RwLock},
    anyhow::{Result, anyhow},
    windexer_common::Block,
    solana_sdk::pubkey::Pubkey,
    windexer_jito_staking::{
        StakingManager,
        staking::OperatorStats,
    },
    crate::consensus::{
        state::ConsensusState,
        validator::ValidatorSet,
        config::ConsensusConfig,
    },
};

pub struct ConsensusProtocol {
    state: Arc<RwLock<ConsensusState>>,
    validator_set: Arc<RwLock<ValidatorSet>>,
    message_tx: mpsc::Sender<ConsensusMessage>,
    staking_manager: Arc<StakingManager>,
}

#[derive(Debug)]
pub enum ConsensusMessage {
    NewBlock(Block),
    Commit(BlockHash),
}

pub type BlockHash = [u8; 32];

impl ConsensusProtocol {
    pub fn new(
        _config: ConsensusConfig,
        state: Arc<RwLock<ConsensusState>>,
        validator_set: Arc<RwLock<ValidatorSet>>,
        staking_manager: Arc<StakingManager>,
    ) -> Self {
        let (message_tx, _message_rx) = mpsc::channel(1000);
        Self {
            state,
            validator_set,
            message_tx,
            staking_manager,
        }
    }

    // Modify handle_block to check stake
    async fn handle_block(&mut self, block: Block) -> Result<()> {
        let mut state = self.state.write().await;
        
        if block.block_height != Some(state.height + 1) {
            return Ok(());
        }

        // Create a 32-byte array for the pubkey
        let mut bytes = [0u8; 32];
        bytes[..8].copy_from_slice(&block.slot.to_le_bytes());
        let proposer = Pubkey::new_from_array(bytes);

        let operator_stats = self.staking_manager
            .get_operator_stats(&proposer)
            .await?;

        if !self.is_stake_sufficient(&operator_stats).await? {
            return Err(anyhow!("Insufficient stake for block proposal"));
        }

        state.current_block = Some(block.clone());
        state.height += 1;
        
        self.message_tx.send(ConsensusMessage::NewBlock(block)).await?;

        Ok(())
    }

    async fn is_stake_sufficient(&self, stats: &OperatorStats) -> Result<bool> {
        Ok(stats.total_stake >= self.staking_manager.config().min_stake)
    }

    // Modify check_consensus to use stake-weighted voting
    async fn check_consensus(&mut self, block_hash: &BlockHash) -> Result<()> {
        let validator_set = self.validator_set.read().await;

        let total_stake = self.get_total_active_stake().await?;
        let vote_stake = self.get_vote_stake(block_hash, &*validator_set).await?;

        // Require 2/3 of stake for consensus
        if vote_stake * 3 > total_stake * 2 {
            self.message_tx
                .send(ConsensusMessage::Commit(*block_hash))
                .await?;
        }

        Ok(())  
    }

    async fn get_total_active_stake(&self) -> Result<u64> {
        let mut total = 0;
        let validators = self.validator_set.read().await;
        
        for validator in validators.get_validators() {
            let stats = self.staking_manager.get_operator_stats(validator).await?;
            total += stats.total_stake;
        }
        Ok(total)
    }

    async fn get_vote_stake(&self, block_hash: &BlockHash, _validator_set: &ValidatorSet) -> Result<u64> {
        let state = self.state.read().await;
        let mut vote_stake = 0;

        if let Some(votes) = state.get_votes(block_hash) {
            for validator in votes.keys() {
                let info = self.staking_manager.get_operator_stats(validator).await?;
                vote_stake += info.total_stake;
            }
        }

        Ok(vote_stake)
    }
}
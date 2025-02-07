// crates/windexer-network/src/consensus/protocol.rs

use {
    super::{ConsensusConfig, state::ConsensusState, validator::ValidatorSet},
    windexer_common::Message,
    windexer_common::MessageType,
    anyhow::Result,
    libp2p::PeerId,
    solana_sdk::pubkey::Pubkey,
    std::sync::Arc,
    tokio::sync::{mpsc, RwLock},
    serde::{Serialize, Deserialize},
};

pub struct ConsensusProtocol {
    config: ConsensusConfig,
    state: Arc<RwLock<ConsensusState>>,
    validator_set: Arc<RwLock<ValidatorSet>>,
    message_tx: mpsc::Sender<ConsensusMessage>,
    message_rx: mpsc::Receiver<ConsensusMessage>,
}

#[derive(Debug)]
pub enum ConsensusMessage {
    NewBlock(Block),
    Vote(Vote),
    Commit(BlockHash),
    ValidatorUpdate(ValidatorSet),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    pub hash: BlockHash,
    pub parent: BlockHash,
    pub height: u64,
    pub proposer: Pubkey,
    pub timestamp: i64,
    pub transactions: Vec<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Vote {
    pub block: BlockHash,
    pub validator: Pubkey,
    pub signature: Vec<u8>,
}

pub type BlockHash = [u8; 32];

impl ConsensusProtocol {
    pub fn new(
        config: ConsensusConfig,
        state: Arc<RwLock<ConsensusState>>,
        validator_set: Arc<RwLock<ValidatorSet>>,
    ) -> Self {
        let (message_tx, message_rx) = mpsc::channel(1000);
        Self {
            config,
            state,
            validator_set,
            message_tx,
            message_rx,
        }
    }

    pub async fn handle_message(&mut self, peer_id: PeerId, message: Message) -> Result<()> {
        match message.message_type {
            MessageType::Block => {
                let block: Block = bincode::deserialize(&message.payload)?;
                self.handle_block(block).await?;
            }
            MessageType::Vote => {
                let vote: Vote = bincode::deserialize(&message.payload)?;
                self.handle_vote(vote).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_block(&mut self, block: Block) -> Result<()> {
        let mut state = self.state.write().await;
        let validator_set = self.validator_set.read().await;

        if !validator_set.is_validator(&block.proposer) {
            return Ok(());
        }

        if block.height != state.height + 1 {
            return Ok(());
        }

        state.add_block(block);
        self.message_tx.send(ConsensusMessage::NewBlock(block)).await?;

        Ok(())
    }

    async fn handle_vote(&mut self, vote: Vote) -> Result<()> {
        let block_hash = vote.block;
        {
            let mut state = self.state.write().await;
            let validator_set = self.validator_set.read().await;

            if !validator_set.is_validator(&vote.validator) {
                return Ok(());
            }

            if state.has_vote(&vote.validator, &vote.block) {
                return Ok(());
            }

            state.add_vote(vote);
        }
        self.check_consensus(&block_hash).await?;
        Ok(())
    }

    async fn check_consensus(&mut self, block_hash: &BlockHash) -> Result<()> {
        let state = self.state.read().await;
        let validator_set = self.validator_set.read().await;

        let total_stake = validator_set.total_stake();
        let vote_stake = state.get_vote_stake(block_hash, &validator_set);

        if vote_stake * 3 > total_stake * 2 {
            self.message_tx
                .send(ConsensusMessage::Commit(*block_hash))
                .await?;
        }

        Ok(())
    }
}

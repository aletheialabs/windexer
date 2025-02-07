// crates/windexer-network/src/consensus/state.rs

use {
    solana_sdk::vote::state::Vote,
    windexer_common::Block,
    super::protocol::BlockHash,
    solana_sdk::pubkey::Pubkey,
    std::collections::HashMap,
};

pub struct ConsensusState {
    pub height: u64,
    pub current_block: Option<Block>,
    pub votes: HashMap<BlockHash, HashMap<Pubkey, Vote>>,
}

impl ConsensusState {
    pub fn new() -> Self {
        Self {
            height: 0,
            current_block: None,
            votes: HashMap::new(),
        }
    }

    pub fn get_votes(&self, block_hash: &BlockHash) -> Option<&HashMap<Pubkey, Vote>> {
        self.votes.get(block_hash)
    }

    pub fn add_vote(&mut self, block_hash: BlockHash, validator: Pubkey, vote: Vote) {
        self.votes.entry(block_hash)
            .or_default()
            .insert(validator, vote);
    }
}
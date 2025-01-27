use {
    super::{protocol::{Block, BlockHash, Vote}, validator::ValidatorSet},
    solana_sdk::pubkey::Pubkey,
    std::collections::{HashMap, HashSet},
};

pub struct ConsensusState {
    pub height: u64,
    pub last_block: Option<BlockHash>,
    blocks: HashMap<BlockHash, Block>,
    votes: HashMap<BlockHash, HashMap<Pubkey, Vote>>,
    pub committed_blocks: Vec<Block>,
}

impl ConsensusState {
    pub fn new() -> Self {
        Self {
            height: 0,
            last_block: None,
            blocks: HashMap::new(),
            votes: HashMap::new(),
            committed_blocks: Vec::new(),
        }
    }

    pub fn add_block(&mut self, block: Block) {
        self.blocks.insert(block.hash, block);
    }

    pub fn add_vote(&mut self, vote: Vote) {
        self.votes
            .entry(vote.block)
            .or_default()
            .insert(vote.validator, vote);
    }

    pub fn has_vote(&self, validator: &Pubkey, block: &BlockHash) -> bool {
        self.votes
            .get(block)
            .map(|votes| votes.contains_key(validator))
            .unwrap_or(false)
    }

    pub fn get_vote_count(&self, block: &BlockHash) -> usize {
        self.votes
            .get(block)
            .map(|votes| votes.len())
            .unwrap_or(0)
    }

    pub fn get_vote_stake(&self, block: &BlockHash, validator_set: &ValidatorSet) -> u64 {
        self.votes
            .get(block)
            .map(|votes| {
                votes
                    .keys()
                    .map(|validator| validator_set.get_stake(validator))
                    .sum()
            })
            .unwrap_or(0)
    }

    pub fn commit_block(&mut self, hash: BlockHash) {
        if let Some(block) = self.blocks.remove(&hash) {
            self.height = block.height;
            self.last_block = Some(hash);
            self.committed_blocks.push(block);
            
            // Clean up old votes
            self.votes.retain(|h, _| h == &hash);
        }
    }
}
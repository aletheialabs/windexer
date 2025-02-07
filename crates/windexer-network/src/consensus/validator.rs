// crates/windexer-network/src/consensus/validator.rs

use {
    solana_sdk::pubkey::Pubkey,
    std::collections::HashMap,
};

#[derive(Debug)]
pub struct ValidatorSet {
    validators: HashMap<Pubkey, ValidatorInfo>,
    total_stake: u64,
}

#[derive(Debug)]
pub struct ValidatorInfo {
    pub stake: u64,
    pub last_vote: Option<i64>,
}

impl ValidatorSet {
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
            total_stake: 0,
        }
    }

    pub fn add_validator(&mut self, pubkey: Pubkey, stake: u64) {
        self.total_stake += stake;
        self.validators.insert(pubkey, ValidatorInfo {
            stake,
            last_vote: None,
        });
    }

    pub fn remove_validator(&mut self, pubkey: &Pubkey) {
        if let Some(info) = self.validators.remove(pubkey) {
            self.total_stake -= info.stake;
        }
    }

    pub fn is_validator(&self, pubkey: &Pubkey) -> bool {
        self.validators.contains_key(pubkey)
    }

    pub fn get_stake(&self, pubkey: &Pubkey) -> u64 {
        self.validators
            .get(pubkey)
            .map(|info| info.stake)
            .unwrap_or(0)
    }

    pub fn total_stake(&self) -> u64 {
        self.total_stake
    }

    pub fn update_vote(&mut self, pubkey: &Pubkey, timestamp: i64) {
        if let Some(info) = self.validators.get_mut(pubkey) {
            info.last_vote = Some(timestamp);
        }
    }

    pub fn get_validators(&self) -> impl Iterator<Item = &Pubkey> {
        self.validators.keys()
    }
}
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use anyhow::Result;

pub struct ValidatorSet {
    validators: HashMap<Pubkey, ValidatorInfo>,
    min_stake: u64,
}

pub struct ValidatorInfo {
    pub pubkey: Pubkey,
    pub stake: u64,
    pub is_active: bool,
    pub last_seen: i64,
}

impl ValidatorSet {
    pub fn new(min_stake: u64) -> Self {
        Self {
            validators: HashMap::new(),
            min_stake,
        }
    }

    pub fn add_validator(&mut self, pubkey: Pubkey, stake: u64) -> Result<()> {
        if stake < self.min_stake {
            return Err(anyhow::anyhow!("Validator stake below minimum threshold"));
        }

        self.validators.insert(pubkey, ValidatorInfo {
            pubkey,
            stake,
            is_active: true,
            last_seen: crate::utils::current_time(),
        });

        Ok(())
    }

    pub fn get_validators(&self) -> Vec<&Pubkey> {
        self.validators.iter()
            .filter(|(_, info)| info.is_active)
            .map(|(key, _)| key)
            .collect()
    }

    pub fn get_validator_info(&self, pubkey: &Pubkey) -> Option<&ValidatorInfo> {
        self.validators.get(pubkey)
    }

    pub fn is_active(&self, pubkey: &Pubkey) -> bool {
        self.validators.get(pubkey)
            .map(|info| info.is_active)
            .unwrap_or(false)
    }
} 
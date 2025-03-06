use serde::{Deserialize, Serialize};
use solana_sdk::{account::Account, pubkey::Pubkey};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountData {
    pub pubkey: Pubkey,
    
    pub owner: Pubkey,
    
    pub lamports: u64,
    
    pub data: Vec<u8>,
    
    pub executable: bool,
    
    pub rent_epoch: u64,
}

impl From<Account> for AccountData {
    fn from(account: Account) -> Self {
        Self {
            pubkey: Pubkey::default(), // Must be set externally
            owner: account.owner,
            lamports: account.lamports,
            data: account.data,
            executable: account.executable,
            rent_epoch: account.rent_epoch,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdate {
    pub slot: u64,
    
    pub account: AccountData,
    
    pub metadata: HashMap<String, String>,
    
    pub timestamp: i64,
}
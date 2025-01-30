use serde::{Deserialize, Serialize};
use solana_sdk::{account::Account, pubkey::Pubkey};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountData {
    /// The account's public key
    pub pubkey: Pubkey,
    
    /// The program that owns this account
    pub owner: Pubkey,
    
    /// The number of lamports assigned to this account
    pub lamports: u64,
    
    /// Data held in this account
    pub data: Vec<u8>,
    
    /// Flag indicating if this account contains a program
    pub executable: bool,
    
    /// The epoch at which this account will next owe rent
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
    /// The slot at which this update occurred
    pub slot: u64,
    
    /// The account data after the update
    pub account: AccountData,
    
    /// Any additional metadata associated with this update
    pub metadata: HashMap<String, String>,
    
    /// Timestamp of when this update was processed
    pub timestamp: i64,
}
//! Account data types
//!
//! This module defines common data structures for working with account data
//! across the wIndexer system.

use {
    solana_sdk::{
        pubkey::Pubkey,
        signature::Signature,
        clock::Slot,
    },
    serde::{Deserialize, Serialize},
    std::fmt::{Debug, Formatter, Result as FmtResult},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct AccountData {
    pub pubkey: Pubkey,
    pub lamports: u64,
    pub owner: Pubkey,
    pub executable: bool,
    pub rent_epoch: u64,
    
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
    pub write_version: u64,
    pub slot: Slot,
    pub is_startup: bool,
    pub transaction_signature: Option<Signature>,
}

impl Debug for AccountData {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("AccountData")
            .field("pubkey", &self.pubkey)
            .field("lamports", &self.lamports)
            .field("owner", &self.owner)
            .field("executable", &self.executable)
            .field("rent_epoch", &self.rent_epoch)
            .field("data_len", &self.data.len())
            .field("write_version", &self.write_version)
            .field("slot", &self.slot)
            .field("is_startup", &self.is_startup)
            .field("transaction_signature", &self.transaction_signature)
            .finish()
    }
}

pub fn deserialize_account<T: serde::de::DeserializeOwned>(
    account_data: &AccountData,
) -> Result<T, bincode::Error> {
    bincode::deserialize(&account_data.data)
}
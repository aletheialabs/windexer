//! Utility functions and helpers

mod crypto;
mod time;
pub mod slot_status;
pub mod transaction_status;

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

pub use crypto::{hash_message, verify_signature};
pub use time::{current_timestamp, duration_since};
pub use slot_status::SerializableSlotStatus;
pub use transaction_status::SerializableTransactionMeta;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerState {
    pub last_processed_slot: u64,
    pub total_accounts: u64,
    pub total_transactions: u64,
    pub last_known_validator: Option<Pubkey>,
}

pub fn pubkey_to_string(pubkey: &Pubkey) -> String {
    pubkey.to_string()
}

pub fn string_to_pubkey(s: &str) -> crate::Result<Pubkey> {
    s.parse::<Pubkey>().map_err(|e: solana_sdk::pubkey::ParsePubkeyError| {
        crate::Error::Other(e.to_string())
    })
}

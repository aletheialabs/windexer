//! Common data types used throughout the windexer system

pub mod account;
pub mod block;
pub mod message;
pub mod transaction;
pub mod helius;

pub use account::AccountData;
pub use block::{BlockData, EntryData, SlotStatusData};
pub use transaction::TransactionData;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerState {
    pub last_processed_slot: u64,
    pub total_accounts: u64,
    pub total_transactions: u64,
}
//! Core domain types used across the system

mod account;
mod block;
mod message;
mod transaction;

pub use account::{AccountData, AccountUpdate};
pub use block::{Block, BlockUpdate};
pub use message::{Message, MessageType, Topic};
pub use transaction::{Transaction, TransactionUpdate};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerState {
    pub last_processed_slot: u64,
    pub total_accounts: u64,
    pub total_transactions: u64,
}
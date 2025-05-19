use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Transaction data structure used for Helius API integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {
    pub signature: String,
    pub slot: u64,
    pub err: bool,
    pub status: u8,  // 0 = failed, 1 = success
    pub fee: u64,
    pub fee_payer: String,
    pub recent_blockhash: String,
    pub accounts: Vec<String>,
    pub log_messages: Vec<String>,
    pub block_time: Option<i64>, 
}

/// Account data structure used for Helius API integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountData {
    pub pubkey: String,
    pub lamports: u64,
    pub owner: String,
    pub executable: bool,
    pub rent_epoch: u64,
    pub data: Vec<u8>,
    pub slot: u64,
    pub write_version: u64,
    pub updated_on: i64,
    pub is_startup: bool,
    pub transaction_signature: Option<String>,
}

/// Block data structure used for Helius API integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockData {
    pub slot: u64,
    pub blockhash: String,
    pub parent_slot: u64,
    pub parent_blockhash: String,
    pub block_time: Option<i64>,
    pub block_height: Option<u64>,
    pub transaction_count: Option<u64>,
    pub status: Option<u8>,  // 0 = unconfirmed, 1 = confirmed, 2 = finalized
    pub leader: Option<String>,
}

/// Subscription response for Helius API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: u64,
}

/// Represents a parsed name account from a Solana name service 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameAccount {
    pub name: String,
    pub parent_name: Option<String>,
    pub owner: String,
    pub class: Option<String>,
    pub expiry: Option<DateTime<Utc>>,
} 
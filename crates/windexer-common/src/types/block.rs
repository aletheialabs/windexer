use serde::{Deserialize, Serialize};
use solana_sdk::hash::Hash;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// The slot number of this block
    pub slot: u64,
    
    /// The block's hash
    pub blockhash: Hash,
    
    /// Parent block's hash
    pub parent_blockhash: Hash,
    
    /// Block height from genesis
    pub block_height: Option<u64>,
    
    /// Unix timestamp recorded by the leader
    pub block_time: Option<i64>,
    
    /// Vector of transaction results
    pub transactions: Vec<TransactionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResult {
    /// The transaction hash
    pub signature: String,
    
    /// Whether the transaction was successful
    pub success: bool,
    
    /// If unsuccessful, the error message
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockUpdate {
    /// The block data
    pub block: Block,
    
    /// Status of this block (processed, confirmed, finalized)
    pub status: BlockStatus,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BlockStatus {
    Processed,
    Confirmed,
    Finalized,
}
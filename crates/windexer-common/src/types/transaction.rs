use serde::{Deserialize, Serialize};
use solana_sdk::{
    instruction::CompiledInstruction,
    message::Message,
    pubkey::Pubkey,
    signature::Signature,
};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// The transaction signature
    pub signature: Signature,
    
    /// The block slot this transaction was processed in
    pub slot: u64,
    
    /// The transaction message containing the instructions
    pub message: Message,
    
    /// The accounts required by this transaction
    pub accounts: Vec<Pubkey>,
    
    /// Program instructions to execute
    pub instructions: Vec<CompiledInstruction>,
    
    /// Recent blockhash this transaction requires
    pub blockhash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionUpdate {
    /// The transaction data
    pub transaction: Transaction,
    
    /// Status of the transaction execution
    pub status: TransactionStatus,
    
    /// Any error that occurred during execution
    pub error: Option<String>,
    
    /// Additional metadata about this transaction
    pub metadata: HashMap<String, String>,
    
    /// Unix timestamp when this update was processed
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TransactionStatus {
    Processing,
    Confirmed,
    Failed,
}
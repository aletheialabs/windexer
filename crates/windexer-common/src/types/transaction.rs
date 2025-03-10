//! Transaction data types
//!
//! This module defines common data structures for working with transaction data
//! across the wIndexer system.

use {
    solana_sdk::{
        signature::Signature,
        clock::Slot,
        message::Message,
    },
    solana_transaction_status::TransactionStatusMeta,
    serde::{Deserialize, Serialize},
    std::fmt::{Debug, Formatter, Result as FmtResult},
    crate::utils::SerializableTransactionMeta,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct TransactionData {
    pub signature: Signature,
    pub slot: Slot,
    pub is_vote: bool,
    pub message: Message,
    pub signatures: Vec<Signature>,
    #[serde(skip_serializing, skip_deserializing)]
    pub meta: TransactionStatusMeta,
    #[serde(rename = "meta")]
    pub serializable_meta: SerializableTransactionMeta,
    pub index: usize,
}

impl Debug for TransactionData {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("TransactionData")
            .field("signature", &self.signature)
            .field("slot", &self.slot)
            .field("is_vote", &self.is_vote)
            .field("message", &"[Message]")
            .field("signatures_count", &self.signatures.len())
            .field("meta", &"[TransactionStatusMeta]")
            .field("index", &self.index)
            .finish()
    }
}
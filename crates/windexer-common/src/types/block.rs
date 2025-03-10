//! Block and entry data types
//!
//! This module defines common data structures for working with block and entry data
//! across the wIndexer system.

use {
    solana_sdk::clock::Slot,
    solana_transaction_status::Reward,
    agave_geyser_plugin_interface::geyser_plugin_interface::SlotStatus,
    serde::{Deserialize, Serialize},
    std::fmt::{Debug, Formatter, Result as FmtResult},
    std::default::Default,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct BlockData {
    pub slot: u64,
    pub parent_slot: Option<u64>,
    #[serde(with = "slot_status_serde")]
    pub status: SlotStatus,
    pub blockhash: Option<String>,
    pub rewards: Option<Vec<Reward>>,
    pub timestamp: Option<i64>,
    pub block_height: Option<u64>,
    pub transaction_count: Option<u64>,
    pub entry_count: u64,
    pub entries: Vec<EntryData>,
    pub parent_blockhash: Option<String>,
}

impl Default for BlockData {
    fn default() -> Self {
        Self {
            slot: 0,
            blockhash: None,
            block_height: None,
            parent_slot: None,
            parent_blockhash: None,
            transaction_count: None,
            timestamp: None,
            rewards: None,
            entry_count: 0,
            entries: Vec::new(),
            status: SlotStatus::Processed,
        }
    }
}

impl Debug for BlockData {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("BlockData")
            .field("slot", &self.slot)
            .field("parent_slot", &self.parent_slot)
            .field("status", &self.status.as_str())
            .field("blockhash", &self.blockhash)
            .field("rewards_count", &self.rewards.as_ref().map_or(0, Vec::len))
            .field("timestamp", &self.timestamp)
            .field("block_height", &self.block_height)
            .field("transaction_count", &self.transaction_count)
            .field("entry_count", &self.entry_count)
            .field("entries_count", &self.entries.len())
            .field("parent_blockhash", &self.parent_blockhash)
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EntryData {
    pub slot: Slot,
    pub index: usize,
    pub num_hashes: u64,
    
    #[serde(with = "serde_bytes")]
    pub hash: Vec<u8>,
    pub executed_transaction_count: u64,
    pub starting_transaction_index: usize,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SlotStatusData {
    pub slot: Slot,
    pub parent: Option<Slot>,
    #[serde(with = "slot_status_serde")]
    pub status: SlotStatus,
}

pub mod slot_status_serde {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(status: &SlotStatus, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(status.as_str())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SlotStatus, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "processed" => Ok(SlotStatus::Processed),
            "confirmed" => Ok(SlotStatus::Confirmed),
            "rooted" => Ok(SlotStatus::Rooted),
            "firstShredReceived" => Ok(SlotStatus::FirstShredReceived),
            "completed" => Ok(SlotStatus::Completed),
            "createdBank" => Ok(SlotStatus::CreatedBank),
            "dead" => Ok(SlotStatus::Dead(String::new())),
            _ => Err(serde::de::Error::custom(format!("Unknown slot status: {}", s))),
        }
    }
}
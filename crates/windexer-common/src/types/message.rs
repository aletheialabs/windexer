//! Network message types
//!
//! This module defines common data structures for network messages
//! across the wIndexer system.

use {
    serde::{Deserialize, Serialize},
    serde_json::Value,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    AccountUpdate,
    TransactionUpdate,
    BlockUpdate,
    EntryUpdate,
    SlotStatusUpdate,
    Control,
    Gossip,
    Heartbeat,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkMessage {
    pub message_type: MessageType,
    pub timestamp: i64,
    pub data: Value,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ControlMessageType {
    JoinRequest,
    JoinResponse,
    LeaveRequest,
    PeerListRequest,
    PeerListResponse,
    StatusRequest,
    StatusResponse,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ControlMessage {
    pub control_type: ControlMessageType,
    pub data: Value,
}
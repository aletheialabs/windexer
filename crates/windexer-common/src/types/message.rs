use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub topic: Topic,
    pub payload: Vec<u8>,
    pub message_type: MessageType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    AccountUpdate,
    BlockUpdate,
    TransactionUpdate,
    Heartbeat,
    PeerAnnouncement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Topic {
    Accounts,
    Blocks,
    Transactions,
    Network,
}
use libp2p::{gossipsub::TopicHash, PeerId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessage {
    pub source: String, // PeerId as string
    pub topics: Vec<String>, // TopicHash as string
    pub payload: Vec<u8>,
    pub data_type: MessageType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    BlockData,
    AccountUpdate,
    Transaction,
    ConsensusVote,
    PeerAnnouncement,
    HeartBeat,
}
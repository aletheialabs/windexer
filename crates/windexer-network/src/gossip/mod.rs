use libp2p::{
    gossipsub::{MessageId, TopicHash},
    PeerId,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

mod message_handler;
mod mesh_manager;
mod topic_handler;

pub use message_handler::MessageHandler;
pub use mesh_manager::MeshManager;
pub use topic_handler::TopicHandler;

#[derive(Clone)]
pub struct GossipConfig {
    pub mesh_n_low: usize,
    pub mesh_n: usize,
    pub mesh_n_high: usize,
    pub mesh_outbound_min: usize,
    pub heartbeat_interval: Duration,
    pub fanout_ttl: Duration,
    pub history_length: usize,
    pub history_gossip: usize,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            mesh_n_low: 4,
            mesh_n: 6,
            mesh_n_high: 12,
            mesh_outbound_min: 3,
            heartbeat_interval: Duration::from_secs(1),
            fanout_ttl: Duration::from_secs(60),
            history_length: 5,
            history_gossip: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessage {
    pub id: MessageId,
    pub source: PeerId,
    pub topics: Vec<TopicHash>,
    pub data: Vec<u8>,
    pub sequence_number: u64,
}

#[derive(Debug)]
pub enum GossipEvent {
    MessageReceived {
        from: PeerId,
        message: GossipMessage,
    },
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    SubscriptionChange {
        peer: PeerId,
        topic: TopicHash,
        subscribed: bool,
    },
}
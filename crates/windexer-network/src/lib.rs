// crates/windexer-network/src/lib.rs

//! windexer-network provides the p2p networking layer for the wIndexer system.
//! It handles peer discovery, message propagation, and network state management
//! using libp2p as the underlying networking stack.

use thiserror::Error;
use std::io;
use libp2p::PeerId;
use solana_sdk::pubkey::Pubkey;

pub mod node;
pub mod gossip;
pub mod consensus;
pub mod metrics;

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Failed to initialize network: {0}")]
    InitializationError(String),

    #[error("Peer connection error: {0}")]
    PeerConnectionError(String),

    #[error("Message propagation error: {0}")]
    MessagePropagationError(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] bincode::Error),

    #[error("Libp2p error: {0}")]
    Libp2pError(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub type Result<T> = std::result::Result<T, NetworkError>;

pub use node::Node;
pub use windexer_common::config::NodeConfig;
pub use gossip::{GossipConfig, GossipMessage, MessageType};
pub use consensus::config::ConsensusConfig;

pub fn init_logging() {
    tracing_subscriber::fmt::init();
}

pub const PROTOCOL_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const MINIMUM_PROTOCOL_VERSION: &str = "0.1.0";

pub struct NetworkPeerId(PeerId);

impl From<PeerId> for NetworkPeerId {
    fn from(peer_id: PeerId) -> Self {
        Self(peer_id)
    }
}

impl From<NetworkPeerId> for Pubkey {
    fn from(peer_id: NetworkPeerId) -> Self {
        let bytes = peer_id.0.to_bytes();
        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes[..32]);
        Pubkey::new_from_array(array)
    }
}
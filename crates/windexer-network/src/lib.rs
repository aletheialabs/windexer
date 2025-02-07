//! windexer-network provides the p2p networking layer for the wIndexer system.
//! It handles peer discovery, message propagation, and network state management
//! using libp2p as the underlying networking stack.

use thiserror::Error;
use std::io;

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
pub use node::NodeConfig;
pub use gossip::{GossipConfig, GossipMessage, MessageType};
pub use consensus::ConsensusConfig;

pub fn init_logging() {
    tracing_subscriber::fmt::init();
}

pub const PROTOCOL_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const MINIMUM_PROTOCOL_VERSION: &str = "0.1.0";
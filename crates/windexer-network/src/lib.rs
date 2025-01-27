//! windexer-network provides the p2p networking layer for the wIndexer system
//! It handles peer discovery, message propagation, and network state management
//! using libp2p as the underlying networking stack.

pub mod node;
pub mod gossip;
pub mod consensus;
pub mod metrics;

use thiserror::Error;

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
}

pub type Result<T> = std::result::Result<T, NetworkError>;

// Re-export key types
pub use node::Node;
pub use gossip::GossipConfig;
pub use consensus::ConsensusConfig;
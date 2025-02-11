//! This is the windexer-common crate - provides shared utilities and types
pub mod config;
pub mod errors;
pub mod types;
pub mod utils;
pub mod crypto;

// Re-export commonly used types
pub use config::{IndexerConfig, NetworkConfig, StoreConfig};
pub use errors::{Error, Result};
pub use types::*;
pub use crypto::SerializableKeypair;

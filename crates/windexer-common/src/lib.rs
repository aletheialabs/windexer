//! This is the windexer-common crate - provides shared utilities and types
pub mod config;
pub mod errors;
pub mod types;
pub mod utils;

// Re-export commonly used types
pub use config::{IndexerConfig, NetworkConfig, StoreConfig};
pub use errors::{Error, Result};
pub use types::*;
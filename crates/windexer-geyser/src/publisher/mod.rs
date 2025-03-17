// crates/windexer-geyser/src/publisher/mod.rs

//! Data publishing module
//!
//! This module contains the interfaces and implementations for publishing processed data
//! to external consumers.

mod network;
mod null;

pub use network::NetworkPublisher;
pub use null::NullPublisher;

use {
    crate::metrics::Metrics,
    anyhow::Result,
    std::sync::Arc,
    windexer_common::types::{
        account::AccountData,
        transaction::TransactionData,
        block::BlockData,
        block::EntryData,
    },
};

#[derive(Clone)]
pub struct PublisherConfig {
    pub network_addr: String,
    pub network_bootstrap_peers: Vec<String>,
    pub solana_rpc_url: Option<String>,
    pub batch_size: usize,
    pub metrics: Arc<Metrics>,
    pub validator_id: Option<String>,
}

impl PublisherConfig {
    pub fn new(
        network_addr: String,
        network_bootstrap_peers: Vec<String>,
        solana_rpc_url: Option<String>,
        batch_size: usize,
        metrics: Arc<Metrics>,
        validator_id: Option<String>,
    ) -> Self {
        Self {
            network_addr,
            network_bootstrap_peers,
            solana_rpc_url,
            batch_size,
            metrics,
            validator_id,
        }
    }
}

pub trait Publisher: Send + Sync + 'static {
    fn publish_accounts(&self, accounts: &[AccountData]) -> Result<()>;
    fn publish_transactions(&self, transactions: &[TransactionData]) -> Result<()>;
    fn publish_block(&self, block: BlockData) -> Result<()>;
    fn publish_entries(&self, entries: &[EntryData]) -> Result<()>;
}
// crates/windexer-geyser/src/metrics.rs

//! Plugin metrics
//!
//! This module contains the metrics for the wIndexer Geyser plugin.

use {
    std::{
        fmt::{Debug, Formatter, Result as FmtResult},
        sync::atomic::{AtomicU64, Ordering},
    },
};

/// Plugin metrics
pub struct Metrics {
    pub account_updates: AtomicU64,
    pub account_update_errors: AtomicU64,
    pub transaction_updates: AtomicU64,
    pub transaction_update_errors: AtomicU64,
    pub block_updates: AtomicU64,
    pub block_update_errors: AtomicU64,
    pub entry_updates: AtomicU64,
    pub entry_updates_errors: AtomicU64,
    pub account_batches_published: AtomicU64,
    pub account_publish_errors: AtomicU64,
    pub transaction_batches_published: AtomicU64,
    pub transaction_publish_errors: AtomicU64,
    pub blocks_published: AtomicU64,
    pub block_publish_errors: AtomicU64,
    pub entry_batches_published: AtomicU64,
    pub entry_publish_errors: AtomicU64,
}

impl Metrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self {
            account_updates: AtomicU64::new(0),
            account_update_errors: AtomicU64::new(0),
            transaction_updates: AtomicU64::new(0),
            transaction_update_errors: AtomicU64::new(0),
            block_updates: AtomicU64::new(0),
            block_update_errors: AtomicU64::new(0),
            entry_updates: AtomicU64::new(0),
            entry_updates_errors: AtomicU64::new(0),
            account_batches_published: AtomicU64::new(0),
            account_publish_errors: AtomicU64::new(0),
            transaction_batches_published: AtomicU64::new(0),
            transaction_publish_errors: AtomicU64::new(0),
            blocks_published: AtomicU64::new(0),
            block_publish_errors: AtomicU64::new(0),
            entry_batches_published: AtomicU64::new(0),
            entry_publish_errors: AtomicU64::new(0),
        }
    }
}

impl Debug for Metrics {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("Metrics")
            .field("account_updates", &self.account_updates.load(Ordering::Relaxed))
            .field("account_update_errors", &self.account_update_errors.load(Ordering::Relaxed))
            .field("transaction_updates", &self.transaction_updates.load(Ordering::Relaxed))
            .field("transaction_update_errors", &self.transaction_update_errors.load(Ordering::Relaxed))
            .field("block_updates", &self.block_updates.load(Ordering::Relaxed))
            .field("block_update_errors", &self.block_update_errors.load(Ordering::Relaxed))
            .field("entry_updates", &self.entry_updates.load(Ordering::Relaxed))
            .field("entry_updates_errors", &self.entry_updates_errors.load(Ordering::Relaxed))
            .field("account_batches_published", &self.account_batches_published.load(Ordering::Relaxed))
            .field("account_publish_errors", &self.account_publish_errors.load(Ordering::Relaxed))
            .field("transaction_batches_published", &self.transaction_batches_published.load(Ordering::Relaxed))
            .field("transaction_publish_errors", &self.transaction_publish_errors.load(Ordering::Relaxed))
            .field("blocks_published", &self.blocks_published.load(Ordering::Relaxed))
            .field("block_publish_errors", &self.block_publish_errors.load(Ordering::Relaxed))
            .field("entry_batches_published", &self.entry_batches_published.load(Ordering::Relaxed))
            .field("entry_publish_errors", &self.entry_publish_errors.load(Ordering::Relaxed))
            .finish()
    }
}
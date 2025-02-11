// crates/windexer-network/src/metrics.rs

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub struct Metrics {
    connected_peers: AtomicU64,
    valid_messages: AtomicU64,
    invalid_messages: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            connected_peers: AtomicU64::new(0),
            valid_messages: AtomicU64::new(0),
            invalid_messages: AtomicU64::new(0),
        }
    }

    pub fn increment_valid_messages(&self) {
        self.valid_messages.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_invalid_messages(&self) {
        self.invalid_messages.fetch_add(1, Ordering::Relaxed);
    }

    pub fn set_connected_peers(&self, count: u64) {
        self.connected_peers.store(count, Ordering::Relaxed);
    }
}
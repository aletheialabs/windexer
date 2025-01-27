use std::sync::atomic::{AtomicU64, Ordering};

pub struct Metrics {
    connected_peers: AtomicU64,
    messages_received: AtomicU64,
    messages_sent: AtomicU64,
    consensus_rounds: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            connected_peers: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            consensus_rounds: AtomicU64::new(0),
        }
    }

    pub fn increment_peers(&self) {
        self.connected_peers.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement_peers(&self) {
        self.connected_peers.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_message_received(&self) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_message_sent(&self) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_consensus_round(&self) {
        self.consensus_rounds.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_metrics(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            connected_peers: self.connected_peers.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            consensus_rounds: self.consensus_rounds.load(Ordering::Relaxed),
        }
    }
}

pub struct MetricsSnapshot {
    pub connected_peers: u64,
    pub messages_received: u64,
    pub messages_sent: u64,
    pub consensus_rounds: u64,
}
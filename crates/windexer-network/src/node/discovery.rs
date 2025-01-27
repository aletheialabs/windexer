use libp2p::PeerId;
use std::collections::HashSet;
use tokio::sync::mpsc;

/// Discovery service for finding peers
pub struct Discovery {
    bootstrap_peers: HashSet<String>,
    events_tx: mpsc::Sender<PeerId>,
    events_rx: mpsc::Receiver<PeerId>,
}

impl Discovery {
    pub fn new(bootstrap_peers: Vec<String>) -> Self {
        let (events_tx, events_rx) = mpsc::channel(32);
        Self {
            bootstrap_peers: bootstrap_peers.into_iter().collect(),
            events_tx,
            events_rx,
        }
    }

    pub async fn next(&mut self) -> Option<PeerId> {
        self.events_rx.recv().await
    }
}
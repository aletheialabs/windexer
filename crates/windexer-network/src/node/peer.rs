use libp2p::{Multiaddr, PeerId};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Manages peer connections and state
pub struct PeerManager {
    peers: HashMap<PeerId, PeerInfo>,
}

struct PeerInfo {
    addr: Multiaddr,
    last_seen: Instant,
    connection_attempts: u32,
}

impl PeerManager {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
        }
    }

    pub fn add_peer(&mut self, peer_id: PeerId, addr: Multiaddr) {
        self.peers.insert(peer_id, PeerInfo {
            addr,
            last_seen: Instant::now(),
            connection_attempts: 0,
        });
    }

    pub fn get_peer_addr(&self, peer_id: &PeerId) -> Option<&Multiaddr> {
        self.peers.get(peer_id).map(|info| &info.addr)
    }

    pub fn mark_seen(&mut self, peer_id: &PeerId) {
        if let Some(info) = self.peers.get_mut(peer_id) {
            info.last_seen = Instant::now();
        }
    }
}
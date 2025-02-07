// crates/windexer-network/src/node/peer.rs

use libp2p::{Multiaddr, PeerId};
use std::collections::HashMap;
use std::time::{Duration, Instant};

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

    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        self.peers.remove(peer_id);
    }

    pub fn is_known_peer(&self, peer_id: &PeerId) -> bool {
        self.peers.contains_key(peer_id)
    }

    pub fn get_peer_addr(&self, peer_id: &PeerId) -> Option<&Multiaddr> {
        self.peers.get(peer_id).map(|info| &info.addr)
    }

    pub fn mark_peer_seen(&mut self, peer_id: &PeerId) {
        if let Some(info) = self.peers.get_mut(peer_id) {
            info.last_seen = Instant::now();
        }
    }

    pub fn get_stale_peers(&self, timeout: Duration) -> Vec<PeerId> {
        let now = Instant::now();
        self.peers
            .iter()
            .filter(|(_, info)| now.duration_since(info.last_seen) > timeout)
            .map(|(peer_id, _)| peer_id.clone())
            .collect()
    }
}
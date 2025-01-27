use super::GossipConfig;
use anyhow::Result;
use libp2p::{PeerId, gossipsub::TopicHash};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info};

pub struct MeshManager {
    config: GossipConfig,
    mesh_peers: HashMap<TopicHash, HashSet<PeerId>>,
    fanout: HashMap<TopicHash, HashSet<PeerId>>,
    peer_topics: HashMap<PeerId, HashSet<TopicHash>>,
}

impl MeshManager {
    pub fn new(config: GossipConfig) -> Self {
        Self {
            config,
            mesh_peers: HashMap::new(),
            fanout: HashMap::new(),
            peer_topics: HashMap::new(),
        }
    }

    pub fn add_peer_to_mesh(&mut self, peer: PeerId, topic: TopicHash) -> Result<()> {
        self.mesh_peers
            .entry(topic.clone())
            .or_default()
            .insert(peer.clone());

        self.peer_topics
            .entry(peer)
            .or_default()
            .insert(topic.clone());

        self.check_mesh_size(&topic);
        Ok(())
    }

    pub fn remove_peer_from_mesh(&mut self, peer: &PeerId, topic: &TopicHash) {
        if let Some(peers) = self.mesh_peers.get_mut(topic) {
            peers.remove(peer);
        }

        if let Some(topics) = self.peer_topics.get_mut(peer) {
            topics.remove(topic);
        }
    }

    pub fn get_mesh_peers(&self, topic: &TopicHash) -> HashSet<PeerId> {
        self.mesh_peers.get(topic).cloned().unwrap_or_default()
    }

    fn check_mesh_size(&mut self, topic: &TopicHash) {
        if let Some(peers) = self.mesh_peers.get(topic) {
            let size = peers.len();
            
            if size > self.config.mesh_n_high {
                debug!("Mesh size too high for topic {:?}: {}", topic, size);
                self.prune_mesh_peers(topic);
            } else if size < self.config.mesh_n_low {
                debug!("Mesh size too low for topic {:?}: {}", topic, size);
                self.graft_mesh_peers(topic);
            }
        }
    }

    fn prune_mesh_peers(&mut self, topic: &TopicHash) {
        if let Some(peers) = self.mesh_peers.get_mut(topic) {
            while peers.len() > self.config.mesh_n {
                if let Some(peer) = peers.iter().next().cloned() {
                    peers.remove(&peer);
                    if let Some(topics) = self.peer_topics.get_mut(&peer) {
                        topics.remove(topic);
                    }
                }
            }
        }
    }

    fn graft_mesh_peers(&mut self, topic: &TopicHash) {
        if let Some(fanout_peers) = self.fanout.get(topic) {
            for peer in fanout_peers.iter() {
                if let Some(mesh_peers) = self.mesh_peers.get_mut(topic) {
                    if mesh_peers.len() >= self.config.mesh_n {
                        break;
                    }
                    if !mesh_peers.contains(peer) {
                        mesh_peers.insert(peer.clone());
                        self.peer_topics
                            .entry(peer.clone())
                            .or_default()
                            .insert(topic.clone());
                    }
                }
            }
        }
    }
}
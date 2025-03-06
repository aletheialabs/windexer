//! P2P Propagator for distributing Solana data across the wIndexer network.
//!
//! This module implements peer-to-peer data propagation using libp2p, enabling the
//! wIndexer network to distribute validator data efficiently across multiple nodes.
//! It supports automatic peer discovery, mesh network formation, and efficient routing.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use futures::prelude::*;
use futures::StreamExt;
use libp2p::{
    core::{
        identity::Keypair,
        muxing::StreamMuxerBox,
        transport::Boxed,
        upgrade::{self, Version},
    },
    gossipsub::{
        self, Gossipsub, GossipsubConfig, GossipsubConfigBuilder, 
        GossipsubEvent, MessageAuthenticity, MessageId, TopicHash,
    },
    identify::{Identify, IdentifyConfig, IdentifyEvent},
    kad::{record::store::MemoryStore, Kademlia, KademliaConfig, KademliaEvent},
    mdns::{Mdns, MdnsConfig, MdnsEvent},
    mplex::MplexConfig,
    noise::{NoiseConfig, X25519Spec},
    swarm::{Swarm, SwarmBuilder, SwarmEvent},
    tcp::TokioTcpConfig,
    Transport, PeerId, NetworkBehaviour,
};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{self, Receiver, Sender};

use agave_geyser_plugin_interface::geyser_plugin_interface::{
    GeyserPluginError, Result as PluginResult,
};

use crate::simd_processing::ProcessedData;

// Custom network behavior combining various protocols
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "P2PEvent")]
struct P2PBehaviour {
    // GossipSub for topic-based message propagation
    gossipsub: Gossipsub,
    // Kademlia for distributed peer discovery and routing
    kademlia: Kademlia<MemoryStore>,
    // MDNS for local peer discovery
    mdns: Mdns,
    // Identify for peer information exchange
    identify: Identify,
}

// Event types for network behavior
#[derive(Debug)]
enum P2PEvent {
    Gossipsub(GossipsubEvent),
    Kademlia(KademliaEvent),
    Mdns(MdnsEvent),
    Identify(IdentifyEvent),
}

impl From<GossipsubEvent> for P2PEvent {
    fn from(event: GossipsubEvent) -> Self {
        P2PEvent::Gossipsub(event)
    }
}

impl From<KademliaEvent> for P2PEvent {
    fn from(event: KademliaEvent) -> Self {
        P2PEvent::Kademlia(event)
    }
}

impl From<MdnsEvent> for P2PEvent {
    fn from(event: MdnsEvent) -> Self {
        P2PEvent::Mdns(event)
    }
}

impl From<IdentifyEvent> for P2PEvent {
    fn from(event: IdentifyEvent) -> Self {
        P2PEvent::Identify(event)
    }
}

// Message types for internal communication
enum PropagatorMessage {
    Account(ProcessedData, u64),
    Transaction(ProcessedData, u64),
    Block(ProcessedData),
    Slot(ProcessedData, u64),
    Entry(ProcessedData),
    Shutdown,
}

/// Propagator for distributing data across the p2p network
#[derive(Debug)]
pub struct P2PPropagator {
    /// Sender for propagator messages
    message_sender: Arc<Mutex<Sender<PropagatorMessage>>>,
    /// Network statistics
    stats: Arc<RwLock<NetworkStats>>,
    /// Runtime handle for the network thread
    _runtime: Arc<Runtime>,
}

/// Network statistics
#[derive(Debug, Default)]
struct NetworkStats {
    /// Number of connected peers
    peers: usize,
    /// Number of messages sent
    messages_sent: usize,
    /// Number of messages received
    messages_received: usize,
    /// Message send success rate
    success_rate: f64,
    /// Last update time
    last_update: Option<Instant>,
    /// Bytes sent
    bytes_sent: usize,
    /// Bytes received
    bytes_received: usize,
    /// Topic stats
    topic_stats: HashMap<String, TopicStats>,
}

/// Stats for a specific topic
#[derive(Debug, Default)]
struct TopicStats {
    /// Messages sent on this topic
    messages_sent: usize,
    /// Messages received on this topic
    messages_received: usize,
    /// Last message time
    last_message: Option<Instant>,
    /// Bytes sent
    bytes_sent: usize,
    /// Bytes received
    bytes_received: usize,
}

impl P2PPropagator {
    /// Create a new P2P propagator
    pub fn new(
        bootstrap_nodes: &[String],
        topics: &[String],
        data_dir: Option<&str>,
    ) -> PluginResult<Self> {
        // Create the runtime for the network thread
        let runtime = Arc::new(Runtime::new().map_err(|e| {
            GeyserPluginError::Custom(Box::new(e))
        })?);
        
        // Create the channel for propagator messages
        let (sender, receiver) = mpsc::channel(1024);
        
        // Create the network stats
        let stats = Arc::new(RwLock::new(NetworkStats::default()));
        
        // Clone handles for the network thread
        let runtime_clone = runtime.clone();
        let stats_clone = stats.clone();
        
        // Start the network thread
        runtime.spawn(async move {
            if let Err(e) = Self::run_network(bootstrap_nodes, topics, receiver, stats_clone, data_dir).await {
                tracing::error!("Error in P2P network thread: {:?}", e);
            }
        });
        
        Ok(Self {
            message_sender: Arc::new(Mutex::new(sender)),
            stats,
            _runtime: runtime_clone,
        })
    }
    
    /// Run the P2P network
    async fn run_network(
        bootstrap_nodes: &[String],
        topics: &[String],
        mut receiver: Receiver<PropagatorMessage>,
        stats: Arc<RwLock<NetworkStats>>,
        data_dir: Option<&str>,
    ) -> PluginResult<()> {
        // Generate a random identity for this node
        let local_key = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        tracing::info!("Local peer id: {}", local_peer_id);
        
        // Create a transport
        let transport = create_transport(&local_key)?;
        
        // Set up GossipSub
        let gossipsub_config = GossipsubConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .mesh_n(8)
            .mesh_n_low(6)
            .mesh_n_high(12)
            .gossip_lazy(3)
            .history_length(10)
            .history_gossip(3)
            .build()
            .map_err(|e| GeyserPluginError::Custom(Box::new(e)))?;
            
        let mut gossipsub = Gossipsub::new(
            MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )
        .map_err(|e| GeyserPluginError::Custom(Box::new(e)))?;
        
        // Subscribe to topics
        let mut topic_hashes = HashMap::new();
        for topic in topics {
            let topic_hash = gossipsub::IdentTopic::new(topic.clone());
            gossipsub.subscribe(&topic_hash)
                .map_err(|e| GeyserPluginError::Custom(Box::new(e)))?;
            topic_hashes.insert(topic.clone(), topic_hash.hash());
            
            // Initialize topic stats
            let mut stats_write = stats.write().unwrap();
            stats_write.topic_stats.insert(
                topic.clone(),
                TopicStats::default(),
            );
            
            tracing::info!("Subscribed to topic: {}", topic);
        }
        
        // Set up Kademlia
        let store = MemoryStore::new(local_peer_id);
        let kademlia_config = KademliaConfig::default();
        let mut kademlia = Kademlia::with_config(local_peer_id, store, kademlia_config);
        
        // Bootstrap Kademlia with known peers
        for peer_addr in bootstrap_nodes {
            if let Ok(addr) = peer_addr.parse() {
                kademlia.add_address(&addr, Duration::from_secs(3600));
                tracing::info!("Added bootstrap node: {}", peer_addr);
            } else {
                tracing::warn!("Invalid bootstrap node address: {}", peer_addr);
            }
        }
        
        // Set up MDNS for local peer discovery
        let mdns = Mdns::new(MdnsConfig::default())
            .await
            .map_err(|e| GeyserPluginError::Custom(Box::new(e)))?;
            
        // Set up Identify
        let identify = Identify::new(
            IdentifyConfig::new("windexer/0.1.0".to_string(), local_key.public())
        );
        
        // Create the network behavior
        let behavior = P2PBehaviour {
            gossipsub,
            kademlia,
            mdns,
            identify,
        };
        
        // Create the swarm
        let mut swarm = SwarmBuilder::new(transport, behavior, local_peer_id)
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build();
        
        // Listen on a random TCP port
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
            .map_err(|e| GeyserPluginError::Custom(Box::new(e)))?;
        
        // Store peer discovery data
        if let Some(dir) = data_dir {
            let path = std::path::Path::new(dir).join("peers.json");
            if path.exists() {
                let peers_json = std::fs::read_to_string(&path)
                    .map_err(|e| GeyserPluginError::Custom(Box::new(e)))?;
                let peers: Vec<String> = serde_json::from_str(&peers_json)
                    .map_err(|e| GeyserPluginError::Custom(Box::new(e)))?;
                
                for peer_addr in peers {
                    if let Ok(addr) = peer_addr.parse() {
                        swarm.behaviour_mut().kademlia.add_address(&addr, Duration::from_secs(3600));
                        tracing::info!("Added discovered peer: {}", peer_addr);
                    }
                }
            }
        }
        
        // Set up statistics update timer
        let mut stats_timer = tokio::time::interval(Duration::from_secs(60));
        
        // Process propagator messages and swarm events
        let mut known_peers = HashSet::new();
        loop {
            tokio::select! {
                // Handle propagator messages
                Some(msg) = receiver.recv() => {
                    match msg {
                        PropagatorMessage::Account(data, slot) => {
                            let topic_hash = topic_hashes.get("accounts").cloned();
                            if let Some(hash) = topic_hash {
                                publish_message(&mut swarm, hash, data, &stats).await?;
                                update_topic_stats(&stats, "accounts", data.len(), 0);
                            }
                        }
                        PropagatorMessage::Transaction(data, slot) => {
                            let topic_hash = topic_hashes.get("transactions").cloned();
                            if let Some(hash) = topic_hash {
                                publish_message(&mut swarm, hash, data, &stats).await?;
                                update_topic_stats(&stats, "transactions", data.len(), 0);
                            }
                        }
                        PropagatorMessage::Block(data) => {
                            let topic_hash = topic_hashes.get("blocks").cloned();
                            if let Some(hash) = topic_hash {
                                publish_message(&mut swarm, hash, data, &stats).await?;
                                update_topic_stats(&stats, "blocks", data.len(), 0);
                            }
                        }
                        PropagatorMessage::Slot(data, slot) => {
                            let topic_hash = topic_hashes.get("slots").cloned();
                            if let Some(hash) = topic_hash {
                                publish_message(&mut swarm, hash, data, &stats).await?;
                                update_topic_stats(&stats, "slots", data.len(), 0);
                            }
                        }
                        PropagatorMessage::Entry(data) => {
                            let topic_hash = topic_hashes.get("entries").cloned();
                            if let Some(hash) = topic_hash {
                                publish_message(&mut swarm, hash, data, &stats).await?;
                                update_topic_stats(&stats, "entries", data.len(), 0);
                            }
                        }
                        PropagatorMessage::Shutdown => {
                            tracing::info!("Shutting down P2P network thread");
                            break;
                        }
                    }
                }
                
                // Handle swarm events
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::Behaviour(P2PEvent::Gossipsub(GossipsubEvent::Message { 
                            propagation_source, 
                            message_id, 
                            message 
                        })) => {
                            tracing::debug!(
                                "Received message from peer {}: {} ({} bytes)",
                                propagation_source,
                                message_id,
                                message.data.len(),
                            );
                            
                            // Update stats
                            {
                                let mut stats_write = stats.write().unwrap();
                                stats_write.messages_received += 1;
                                stats_write.bytes_received += message.data.len();
                                
                                // Update topic stats
                                for (topic, hash) in &topic_hashes {
                                    if message.topic == *hash {
                                        if let Some(topic_stats) = stats_write.topic_stats.get_mut(topic) {
                                            topic_stats.messages_received += 1;
                                            topic_stats.bytes_received += message.data.len();
                                            topic_stats.last_message = Some(Instant::now());
                                        }
                                        break;
                                    }
                                }
                            }
                            
                            // Process received message (in a real implementation, you would 
                            // deserialize and act on the message here)
                        }
                        
                        SwarmEvent::Behaviour(P2PEvent::Mdns(MdnsEvent::Discovered(list))) => {
                            for (peer_id, addr) in list {
                                if !known_peers.contains(&peer_id) {
                                    tracing::info!("Discovered peer via mDNS: {} at {}", peer_id, addr);
                                    swarm.behaviour_mut().kademlia.add_address(&peer_id, addr, Duration::from_secs(3600));
                                    known_peers.insert(peer_id);
                                    
                                    // Update peer count
                                    let mut stats_write = stats.write().unwrap();
                                    stats_write.peers = known_peers.len();
                                }
                            }
                        }
                        
                        SwarmEvent::Behaviour(P2PEvent::Kademlia(KademliaEvent::OutboundQueryCompleted { 
                            result, ..
                        })) => {
                            match result {
                                libp2p::kad::QueryResult::GetProviders(Ok(providers)) => {
                                    for peer in providers.providers {
                                        if !known_peers.contains(&peer) {
                                            tracing::info!("Discovered provider peer: {}", peer);
                                            known_peers.insert(peer);
                                            
                                            // Update peer count
                                            let mut stats_write = stats.write().unwrap();
                                            stats_write.peers = known_peers.len();
                                        }
                                    }
                                }
                                libp2p::kad::QueryResult::GetClosestPeers(Ok(peers)) => {
                                    for peer in peers.peers {
                                        if !known_peers.contains(&peer) {
                                            tracing::info!("Discovered closest peer: {}", peer);
                                            known_peers.insert(peer);
                                            
                                            // Update peer count
                                            let mut stats_write = stats.write().unwrap();
                                            stats_write.peers = known_peers.len();
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        
                        SwarmEvent::Behaviour(P2PEvent::Identify(IdentifyEvent::Received { 
                            peer_id, info, ..
                        })) => {
                            tracing::info!("Identified peer {}: {} ({})", peer_id, info.agent_version, info.protocol_version);
                            
                            // Add addresses to Kademlia
                            for addr in info.listen_addrs {
                                swarm.behaviour_mut().kademlia.add_address(&peer_id, addr, Duration::from_secs(3600));
                            }
                        }
                        
                        SwarmEvent::NewListenAddr { address, .. } => {
                            tracing::info!("Listening on {}", address);
                        }
                        
                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            tracing::info!("Connection closed to peer: {}", peer_id);
                            known_peers.remove(&peer_id);
                            
                            // Update peer count
                            let mut stats_write = stats.write().unwrap();
                            stats_write.peers = known_peers.len();
                        }
                        
                        _ => {}
                    }
                }
                
                // Periodic tasks
                _ = stats_timer.tick() => {
                    // Save discovered peers to disk
                    if let Some(dir) = data_dir {
                        let peers: Vec<String> = known_peers.iter()
                            .filter_map(|peer_id| {
                                swarm.behaviour()
                                    .kademlia
                                    .addresses_of_peer(peer_id)
                                    .into_iter()
                                    .next()
                                    .map(|addr| format!("{}/{}", addr, peer_id))
                            })
                            .collect();
                            
                        if !peers.is_empty() {
                            let path = std::path::Path::new(dir).join("peers.json");
                            let peers_json = serde_json::to_string(&peers)
                                .map_err(|e| GeyserPluginError::Custom(Box::new(e)))?;
                                
                            std::fs::create_dir_all(std::path::Path::new(dir))
                                .map_err(|e| GeyserPluginError::Custom(Box::new(e)))?;
                                
                            std::fs::write(&path, peers_json)
                                .map_err(|e| GeyserPluginError::Custom(Box::new(e)))?;
                                
                            tracing::info!("Saved {} peers to disk", peers.len());
                        }
                    }
                    
                    // Log network stats
                    {
                        let stats_read = stats.read().unwrap();
                        tracing::info!(
                            "Network stats: {} peers, {} sent, {} received, {}% success rate, {} bytes sent, {} bytes received", 
                            stats_read.peers,
                            stats_read.messages_sent,
                            stats_read.messages_received,
                            stats_read.success_rate * 100.0,
                            stats_read.bytes_sent,
                            stats_read.bytes_received,
                        );
                        
                        for (topic, topic_stats) in &stats_read.topic_stats {
                            tracing::info!(
                                "Topic {}: {} sent, {} received, {} bytes sent, {} bytes received",
                                topic,
                                topic_stats.messages_sent,
                                topic_stats.messages_received,
                                topic_stats.bytes_sent,
                                topic_stats.bytes_received,
                            );
                        }
                    }
                    
                    // Bootstrap Kademlia again to find more peers
                    let _ = swarm.behaviour_mut().kademlia.bootstrap();
                }
            }
        }
        
        Ok(())
    }
    
    /// Propagate account data to the network
    pub fn propagate_account(&self, data: ProcessedData, slot: u64) -> PluginResult<()> {
        let sender = self.message_sender.lock().unwrap();
        sender.try_send(PropagatorMessage::Account(data, slot))
            .map_err(|e| GeyserPluginError::Custom(Box::new(e)))
    }
    
    /// Propagate transaction data to the network
    pub fn propagate_transaction(&self, data: ProcessedData, slot: u64) -> PluginResult<()> {
        let sender = self.message_sender.lock().unwrap();
        sender.try_send(PropagatorMessage::Transaction(data, slot))
            .map_err(|e| GeyserPluginError::Custom(Box::new(e)))
    }
    
    /// Propagate block data to the network
    pub fn propagate_block(&self, data: ProcessedData) -> PluginResult<()> {
        let sender = self.message_sender.lock().unwrap();
        sender.try_send(PropagatorMessage::Block(data))
            .map_err(|e| GeyserPluginError::Custom(Box::new(e)))
    }
    
    /// Propagate slot data to the network
    pub fn propagate_slot(&self, data: ProcessedData, slot: u64) -> PluginResult<()> {
        let sender = self.message_sender.lock().unwrap();
        sender.try_send(PropagatorMessage::Slot(data, slot))
            .map_err(|e| GeyserPluginError::Custom(Box::new(e)))
    }
    
    /// Propagate entry data to the network
    pub fn propagate_entry(&self, data: ProcessedData) -> PluginResult<()> {
        let sender = self.message_sender.lock().unwrap();
        sender.try_send(PropagatorMessage::Entry(data))
            .map_err(|e| GeyserPluginError::Custom(Box::new(e)))
    }
    
    /// Get network statistics
    pub fn get_stats(&self) -> (usize, usize, usize, f64) {
        let stats = self.stats.read().unwrap();
        (
            stats.peers,
            stats.messages_sent,
            stats.messages_received,
            stats.success_rate,
        )
    }
}

impl Drop for P2PPropagator {
    fn drop(&mut self) {
        // Send shutdown message to network thread
        if let Ok(sender) = self.message_sender.lock() {
            let _ = sender.try_send(PropagatorMessage::Shutdown);
        }
    }
}

/// Create a libp2p transport with noise encryption and mplex for stream multiplexing
fn create_transport(
    local_key: &Keypair,
) -> PluginResult<Boxed<(PeerId, StreamMuxerBox)>> {
    let transport = TokioTcpConfig::new()
        .nodelay(true)
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(local_key.clone()).into_authenticated())
        .multiplex(MplexConfig::new())
        .boxed();
        
    Ok(transport)
}

/// Publish a message to the network
async fn publish_message(
    swarm: &mut Swarm<P2PBehaviour>,
    topic: TopicHash,
    data: Vec<u8>,
    stats: &Arc<RwLock<NetworkStats>>,
) -> PluginResult<()> {
    match swarm.behaviour_mut().gossipsub.publish(topic, data.clone()) {
        Ok(message_id) => {
            tracing::debug!("Published message: {}", message_id);
            
            // Update stats
            let mut stats_write = stats.write().unwrap();
            stats_write.messages_sent += 1;
            stats_write.bytes_sent += data.len();
            stats_write.success_rate = if stats_write.messages_sent > 0 {
                1.0 // In a real implementation, track failures for a real success rate
            } else {
                0.0
            };
            stats_write.last_update = Some(Instant::now());
            
            Ok(())
        }
        Err(e) => {
            tracing::warn!("Failed to publish message: {}", e);
            Err(GeyserPluginError::Custom(Box::new(e)))
        }
    }
}

/// Update topic statistics
fn update_topic_stats(
    stats: &Arc<RwLock<NetworkStats>>,
    topic: &str,
    bytes_sent: usize,
    bytes_received: usize,
) {
    let mut stats_write = stats.write().unwrap();
    if let Some(topic_stats) = stats_write.topic_stats.get_mut(topic) {
        if bytes_sent > 0 {
            topic_stats.messages_sent += 1;
            topic_stats.bytes_sent += bytes_sent;
        }
        if bytes_received > 0 {
            topic_stats.messages_received += 1;
            topic_stats.bytes_received += bytes_received;
        }
        topic_stats.last_message = Some(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_p2p_propagator_creation() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let propagator = P2PPropagator::new(
                &[],
                &["accounts".to_string(), "transactions".to_string()],
                None,
            ).unwrap();
            
            // Check that the propagator was created successfully
            assert!(propagator.message_sender.lock().unwrap().capacity() > 0);
            
            // Allow time for the network to start
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            // Get stats
            let (peers, sent, received, success_rate) = propagator.get_stats();
            assert_eq!(sent, 0);
            assert_eq!(received, 0);
            assert_eq!(success_rate, 0.0);
        });
    }
}

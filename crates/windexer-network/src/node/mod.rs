use anyhow::Result;
use libp2p::{
    core::muxing::StreamMuxerBox,
    core::transport::Boxed,
    gossipsub::{Gossipsub, GossipsubConfig, GossipsubEvent, MessageAuthenticity, Topic},
    identity::Keypair,
    mdns::{Mdns, MdnsEvent},
    noise,
    swarm::{NetworkBehaviour, Swarm, SwarmBuilder, SwarmEvent},
    tcp::Config as TcpConfig,
    yamux::YamuxConfig,
    PeerId, Transport,
};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

mod discovery;
mod peer;

use discovery::Discovery;
use peer::PeerManager;

#[derive(Clone)]
pub struct NodeConfig {
    pub keypair: Keypair,
    pub listen_addresses: Vec<String>,
    pub bootstrap_peers: Vec<String>,
    pub heartbeat_interval: Duration,
}

pub struct Node {
    swarm: Swarm<WIndexerBehaviour>,
    peer_manager: PeerManager,
    discovery: Discovery,
    _shutdown: mpsc::Receiver<()>,
}

#[derive(NetworkBehaviour)]
struct WIndexerBehaviour {
    gossipsub: Gossipsub,
    mdns: Mdns,
}

impl Node {
    pub async fn new(config: NodeConfig) -> Result<(Self, mpsc::Sender<()>)> {
        let peer_id = PeerId::from(config.keypair.public());
        info!("Local peer id: {peer_id}");

        let transport = libp2p::tcp::tokio::Transport::default()
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(noise::NoiseAuthenticated::xx(&config.keypair)?)
            .multiplex(YamuxConfig::default())
            .boxed();

        let gossipsub_config = GossipsubConfig::default();
        let gossipsub = Gossipsub::new(
            MessageAuthenticity::Signed(config.keypair.clone()),
            gossipsub_config,
        )?;
        let mdns = Mdns::new(Default::default()).await?;

        let behaviour = WIndexerBehaviour {
            gossipsub,
            mdns,
        };

        // Build the swarm
        let swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, peer_id).build();

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        let peer_manager = PeerManager::new();
        let discovery = Discovery::new(config.bootstrap_peers);

        Ok((
            Self {
                swarm,
                peer_manager,
                discovery,
                _shutdown: shutdown_rx,
            },
            shutdown_tx,
        ))
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                swarm_event = self.swarm.next_event() => {
                    match swarm_event {
                        SwarmEvent::Behaviour(event) => {
                            match event {
                                WIndexerBehaviourEvent::Gossipsub(GossipsubEvent::Message { 
                                    message_id,
                                    propagation_source,
                                    message,
                                    ..
                                }) => {
                                    debug!("Got message: {message_id} from {propagation_source}");
                                    self.handle_message(message).await?;
                                }
                                
                                WIndexerBehaviourEvent::Mdns(MdnsEvent::Discovered(peers)) => {
                                    for (peer_id, addr) in peers {
                                        self.peer_manager.add_peer(peer_id.clone(), addr.clone());
                                        self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                                        info!("Discovered peer: {peer_id} at {addr}");
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                
                discovery_event = self.discovery.next() => {
                    if let Some(peer) = discovery_event {
                        self.handle_discovered_peer(peer).await?;
                    }
                }
            }
        }
    }

    async fn handle_message(&mut self, message: Vec<u8>) -> Result<()> {
    // First try to deserialize the message
    let gossip_message: GossipMessage = match bincode::deserialize(&message) {
        Ok(msg) => msg,
        Err(e) => {
            warn!("Failed to deserialize message: {}", e);
            return Ok(());
        }
    };

    // Process based on message type
    match gossip_message.data_type {
        MessageType::BlockData => {
            // Handle block data for indexing
            if let Ok(block_data) = bincode::deserialize(&gossip_message.payload) {
                self.handle_block_data(block_data).await?;
            }
        }
        MessageType::AccountUpdate => {
            // Handle account updates
            if let Ok(account_update) = bincode::deserialize(&gossip_message.payload) {
                self.handle_account_update(account_update).await?;
            }
        }
        MessageType::Transaction => {
            // Handle transaction data
            if let Ok(transaction) = bincode::deserialize(&gossip_message.payload) {
                self.handle_transaction(transaction).await?;
            }
        }
        MessageType::ConsensusVote => {
            // Handle consensus messages
            if let Ok(vote_data) = bincode::deserialize(&gossip_message.payload) {
                self.handle_consensus_vote(vote_data).await?;
            }
        }
        MessageType::PeerAnnouncement => {
            // Handle peer announcements
            if let Ok(peer_info) = bincode::deserialize(&gossip_message.payload) {
                self.handle_peer_announcement(peer_info).await?;
            }
        }
        MessageType::HeartBeat => {
            // Update peer liveliness
            if let Ok(heartbeat) = bincode::deserialize(&gossip_message.payload) {
                self.handle_heartbeat(heartbeat).await?;
            }
        }
    }

    // Forward message to other peers based on gossip protocol
    self.forward_message(&gossip_message).await?;

    Ok(())
}

async fn handle_discovered_peer(&mut self, peer: PeerId) -> Result<()> {
    // Check if we already know this peer
    if self.peer_manager.is_known_peer(&peer) {
        debug!("Rediscovered known peer: {}", peer);
        self.peer_manager.mark_peer_seen(&peer);
        return Ok(());
    }

    // Get peer info if available
    let peer_info = match self.fetch_peer_info(&peer).await {
        Ok(info) => info,
        Err(e) => {
            warn!("Failed to fetch info for peer {}: {}", peer, e);
            return Ok(());
        }
    };

    // Verify peer capabilities
    if !self.verify_peer_capabilities(&peer_info).await? {
        debug!("Peer {} does not meet required capabilities", peer);
        return Ok(());
    }

    // Add to peer manager
    self.peer_manager.add_peer(peer.clone(), peer_info.addr.clone());

    // Subscribe to peer's topics
    for topic in peer_info.topics {
        self.swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
    }

    // Add as explicit peer in gossipsub
    self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);

    // Send our node info to the new peer
    let our_info = self.get_node_info()?;
    self.send_peer_info(peer, our_info).await?;

    // Update metrics
    self.metrics.peer_connected();

    info!("Added new peer {} at {}", peer, peer_info.addr);

    Ok(())
}

// Helper methods for message handling
impl Node {
    async fn handle_block_data(&mut self, block_data: BlockData) -> Result<()> {
        // Validate block data
        if !self.validate_block_data(&block_data).await? {
            warn!("Invalid block data received");
            return Ok(());
        }

        // Process block data
        self.process_block_data(block_data).await?;

        Ok(())
    }

    async fn handle_account_update(&mut self, update: AccountUpdate) -> Result<()> {
        // Validate account update
        if !self.validate_account_update(&update).await? {
            warn!("Invalid account update received");
            return Ok(());
        }

        // Process account update
        self.process_account_update(update).await?;

        Ok(())
    }

    async fn handle_transaction(&mut self, transaction: TransactionInfo) -> Result<()> {
        // Validate transaction
        if !self.validate_transaction(&transaction).await? {
            warn!("Invalid transaction received");
            return Ok(());
        }

        // Process transaction
        self.process_transaction(transaction).await?;

        Ok(())
    }

    async fn handle_consensus_vote(&mut self, vote: ConsensusVote) -> Result<()> {
        // Validate vote
        if !self.validate_consensus_vote(&vote).await? {
            warn!("Invalid consensus vote received");
            return Ok(());
        }

        // Process vote
        self.process_consensus_vote(vote).await?;

        Ok(())
    }

    async fn forward_message(&mut self, message: &GossipMessage) -> Result<()> {
        let peers = self.peer_manager.get_gossip_peers();
        for peer in peers {
            if message.source != peer {
                self.send_message(peer, message.clone()).await?;
            }
        }
        Ok(())
    }

    async fn fetch_peer_info(&self, peer: &PeerId) -> Result<PeerInfo> {
        // Request peer information through libp2p
        let info = self.swarm.behaviour()
            .gossipsub
            .get_peer_info(peer)
            .ok_or_else(|| anyhow::anyhow!("Failed to get peer info"))?;

        Ok(PeerInfo {
            addr: info.connected_addr().cloned()
                .ok_or_else(|| anyhow::anyhow!("No address for peer"))?,
            topics: info.topics().cloned().collect(),
            protocol_version: info.protocol_version(),
            agent_version: info.agent_version().to_string(),
        })
    }

    async fn verify_peer_capabilities(&self, info: &PeerInfo) -> Result<bool> {
        // Check minimum protocol version
        if info.protocol_version < MINIMUM_PROTOCOL_VERSION {
            return Ok(false);
        }

        // Check required topics
        let required_topics = self.get_required_topics();
        for topic in required_topics {
            if !info.topics.contains(&topic) {
                return Ok(false);
            }
        }

        Ok(true)
    }
}
}
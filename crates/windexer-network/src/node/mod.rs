use anyhow::Result;
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed},
    gossipsub::{self, Gossipsub, GossipsubConfig, GossipsubEvent, MessageAuthenticity},
    identity::Keypair,
    mdns::{tokio::Behaviour as Mdns, Event as MdnsEvent},
    noise,
    swarm::{NetworkBehaviour, Swarm, SwarmBuilder, SwarmEvent},
    tcp::Config as TcpConfig,
    yamux, Multiaddr, PeerId, Transport,
};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::gossip::{GossipMessage, MessageType};
use crate::metrics::Metrics;

pub struct NodeConfig {
    pub keypair: Keypair,
    pub listen_addresses: Vec<String>, 
    pub bootstrap_peers: Vec<String>,
    pub heartbeat_interval: Duration,
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "WIndexerEvent")]
struct WIndexerBehaviour {
    gossipsub: Gossipsub,
    mdns: Mdns,
}

#[derive(Debug)]
enum WIndexerEvent {
    Gossipsub(GossipsubEvent),
    Mdns(MdnsEvent),
}

pub struct Node {
    swarm: Swarm<WIndexerBehaviour>,
    metrics: Metrics,
    _shutdown: mpsc::Receiver<()>,
}

impl Node {
    pub async fn new(config: NodeConfig) -> Result<(Self, mpsc::Sender<()>)> {
        let peer_id = PeerId::from(config.keypair.public());
        info!("Local peer id: {peer_id}");

        // Setup transport with noise encryption and yamux multiplexing
        let transport = libp2p::tcp::tokio::Transport::default()
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(noise::NoiseAuthenticated::xx(&config.keypair)?)
            .multiplex(yamux::Config::default())
            .boxed();

        // Configure gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(config.heartbeat_interval)
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .expect("Valid config");

        let gossipsub = Gossipsub::new(
            MessageAuthenticity::Signed(config.keypair),
            gossipsub_config,
        )?;

        // Setup MDNS
        let mdns = Mdns::new(Default::default(), peer_id).await?;

        // Combine behaviours
        let behaviour = WIndexerBehaviour {
            gossipsub,
            mdns,
        };

        // Build the swarm
        let swarm = SwarmBuilder::with_executor(
            transport,
            behaviour,
            peer_id,
            tokio::runtime::Handle::current(),
        )
        .build();

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        Ok((
            Self {
                swarm,
                metrics: Metrics::new(),
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
    let gossip_message: GossipMessage = match bincode::deserialize(&message) {
        Ok(msg) => msg,
        Err(e) => {
            warn!("Failed to deserialize message: {}", e);
            return Ok(());
        }
    };

    match gossip_message.data_type {
        MessageType::BlockData => {
            if let Ok(block_data) = bincode::deserialize(&gossip_message.payload) {
                self.handle_block_data(block_data).await?;
            }
        }
        MessageType::AccountUpdate => {
            if let Ok(account_update) = bincode::deserialize(&gossip_message.payload) {
                self.handle_account_update(account_update).await?;
            }
        }
        MessageType::Transaction => {
            if let Ok(transaction) = bincode::deserialize(&gossip_message.payload) {
                self.handle_transaction(transaction).await?;
            }
        }
        MessageType::ConsensusVote => {
            if let Ok(vote_data) = bincode::deserialize(&gossip_message.payload) {
                self.handle_consensus_vote(vote_data).await?;
            }
        }
        MessageType::PeerAnnouncement => {
            if let Ok(peer_info) = bincode::deserialize(&gossip_message.payload) {
                self.handle_peer_announcement(peer_info).await?;
            }
        }
        MessageType::HeartBeat => {
            if let Ok(heartbeat) = bincode::deserialize(&gossip_message.payload) {
                self.handle_heartbeat(heartbeat).await?;
            }
        }
    }

    self.forward_message(&gossip_message).await?;

    Ok(())
}

async fn handle_discovered_peer(&mut self, peer: PeerId) -> Result<()> {
    if self.peer_manager.is_known_peer(&peer) {
        debug!("Rediscovered known peer: {}", peer);
        self.peer_manager.mark_peer_seen(&peer);
        return Ok(());
    }
    let peer_info = match self.fetch_peer_info(&peer).await {
        Ok(info) => info,
        Err(e) => {
            warn!("Failed to fetch info for peer {}: {}", peer, e);
            return Ok(());
        }
    };

    if !self.verify_peer_capabilities(&peer_info).await? {
        debug!("Peer {} does not meet required capabilities", peer);
        return Ok(());
    }

    self.peer_manager.add_peer(peer.clone(), peer_info.addr.clone());

    for topic in peer_info.topics {
        self.swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
    }

    self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);

    let our_info = self.get_node_info()?;
    self.send_peer_info(peer, our_info).await?;

    self.metrics.peer_connected();

    info!("Added new peer {} at {}", peer, peer_info.addr);

    Ok(())
}

impl Node {
    async fn handle_block_data(&mut self, block_data: BlockData) -> Result<()> {
        if !self.validate_block_data(&block_data).await? {
            warn!("Invalid block data received");
            return Ok(());
        }

        self.process_block_data(block_data).await?;

        Ok(())
    }

    async fn handle_account_update(&mut self, update: AccountUpdate) -> Result<()> {
        if !self.validate_account_update(&update).await? {
            warn!("Invalid account update received");
            return Ok(());
        }

        self.process_account_update(update).await?;

        Ok(())
    }

    async fn handle_transaction(&mut self, transaction: TransactionInfo) -> Result<()> {
        if !self.validate_transaction(&transaction).await? {
            warn!("Invalid transaction received");
            return Ok(());
        }

        self.process_transaction(transaction).await?;

        Ok(())
    }

    async fn handle_consensus_vote(&mut self, vote: ConsensusVote) -> Result<()> {
        if !self.validate_consensus_vote(&vote).await? {
            warn!("Invalid consensus vote received");
            return Ok(());
        }

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
        if info.protocol_version < MINIMUM_PROTOCOL_VERSION {
            return Ok(false);
        }

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
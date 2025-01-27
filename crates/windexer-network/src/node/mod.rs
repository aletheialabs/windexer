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

/// Configuration for the network node
#[derive(Clone)]
pub struct NodeConfig {
    /// Local keypair for node identity
    pub keypair: Keypair,
    /// Listen addresses
    pub listen_addresses: Vec<String>,
    /// Bootstrap peers
    pub bootstrap_peers: Vec<String>,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
}

/// Core networking node for wIndexer
pub struct Node {
    /// The libp2p swarm
    swarm: Swarm<WIndexerBehaviour>,
    /// Peer manager
    peer_manager: PeerManager,
    /// Discovery service
    discovery: Discovery,
    /// Shutdown channel
    _shutdown: mpsc::Receiver<()>,
}

#[derive(NetworkBehaviour)]
struct WIndexerBehaviour {
    gossipsub: Gossipsub,
    mdns: Mdns,
}

impl Node {
    /// Create a new network node
    pub async fn new(config: NodeConfig) -> Result<(Self, mpsc::Sender<()>)> {
        let peer_id = PeerId::from(config.keypair.public());
        info!("Local peer id: {peer_id}");

        // Set up transport
        let transport = libp2p::tcp::tokio::Transport::default()
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(noise::NoiseAuthenticated::xx(&config.keypair)?)
            .multiplex(YamuxConfig::default())
            .boxed();

        // Configure gossipsub
        let gossipsub_config = GossipsubConfig::default();
        let gossipsub = Gossipsub::new(
            MessageAuthenticity::Signed(config.keypair.clone()),
            gossipsub_config,
        )?;

        // Set up mDNS for peer discovery
        let mdns = Mdns::new(Default::default()).await?;

        // Create the network behaviour
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

    /// Start the network node
    pub async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                swarm_event = self.swarm.next_event() => {
                    match swarm_event {
                        SwarmEvent::Behaviour(event) => {
                            match event {
                                // Handle gossipsub events
                                WIndexerBehaviourEvent::Gossipsub(GossipsubEvent::Message { 
                                    message_id,
                                    propagation_source,
                                    message,
                                    ..
                                }) => {
                                    debug!("Got message: {message_id} from {propagation_source}");
                                    // Process message
                                    self.handle_message(message).await?;
                                }
                                
                                // Handle peer discovery via mDNS
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
        // Message handling logic here
        Ok(())
    }

    async fn handle_discovered_peer(&mut self, peer: PeerId) -> Result<()> {
        // Peer handling logic here
        Ok(())
    }
}
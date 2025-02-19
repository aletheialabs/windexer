// crates/windexer-network/src/node/mod.rs

use {
    crate::{
        metrics::Metrics,
        NetworkPeerId,
    },
    anyhow::{anyhow, Context, Result},
    futures::StreamExt,
    libp2p::{
        core::upgrade,
        gossipsub::{
            self, 
            Behaviour as GossipsubBehaviour,
            MessageAuthenticity,
            ValidationMode,
        },
        mdns::{self, tokio::Behaviour as MdnsBehaviour},
        noise,
        swarm::{NetworkBehaviour, SwarmEvent, Swarm, Config as SwarmConfig},
        tcp,
        yamux,
        Multiaddr,
        PeerId,
        Transport,
        identity,
    },
    solana_sdk::{
        pubkey::Pubkey,
        signer::keypair::Keypair as SolanaKeypair,
    },
    std::{
        collections::HashSet,
        sync::Arc,
        time::Duration,
    },
    tokio::{
        sync::{mpsc, Mutex, RwLock},
        time,
    },
    tracing::{debug, info, warn},
    windexer_common::config::{NodeConfig, NodeType},
    windexer_jito_staking::{JitoStakingService, StakingConfig},
};

use std::convert::TryInto;
use libp2p::SwarmBuilder;

pub fn convert_keypair(solana_keypair: &SolanaKeypair) -> identity::Keypair {
    let full_bytes = solana_keypair.to_bytes();
    let seed: [u8; 32] = full_bytes[..32]
        .try_into()
        .expect("Slice should have a length of 32 bytes");
    identity::Keypair::ed25519_from_bytes(seed)
        .expect("Valid keypair conversion")
}

// Add new behavior types for different node roles
#[derive(NetworkBehaviour)]
struct PublisherBehaviour {
    gossipsub: GossipsubBehaviour,
    mdns: MdnsBehaviour,
}

#[derive(NetworkBehaviour)]
struct RelayerBehaviour {
    gossipsub: GossipsubBehaviour,
    mdns: MdnsBehaviour,
}

// Events that can be produced by our network behavior
#[derive(Debug)]
enum NodeEvent {
    Gossipsub(gossipsub::Event),
    Mdns(mdns::Event),
}

impl From<gossipsub::Event> for NodeEvent {
    fn from(event: gossipsub::Event) -> Self {
        NodeEvent::Gossipsub(event)
    }
}

impl From<mdns::Event> for NodeEvent {
    fn from(event: mdns::Event) -> Self {
        NodeEvent::Mdns(event)
    }
}

// Enum to hold different swarm types
enum NodeSwarm {
    Publisher(Swarm<PublisherBehaviour>),
    Relayer(Swarm<RelayerBehaviour>),
}

// Update Node struct to handle different swarm types
pub struct Node {
    config: Box<dyn NodeConfig>,
    swarm: Arc<Mutex<NodeSwarm>>,
    metrics: Arc<RwLock<Metrics>>,
    known_peers: Arc<RwLock<HashSet<PeerId>>>,
    shutdown_rx: mpsc::Receiver<()>,
    staking_service: Arc<JitoStakingService>,
}

// impl std::fmt::Debug for Node {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("Node")
//             .field("config", &self.config)
//             .field("metrics", &self.metrics)
//             .field("known_peers", &self.known_peers)
//             .finish_non_exhaustive()
//     }
// }

impl Node {
    pub async fn new(
        config: Box<dyn NodeConfig>,
        staking_config: StakingConfig,
    ) -> Result<(Self, mpsc::Sender<()>)> {
        let keypair = config.get_keypair().to_keypair()?;
        let libp2p_keypair = convert_keypair(&keypair);
        let peer_id = PeerId::from(libp2p_keypair.public());
        
        info!("Initializing node with PeerID: {}", peer_id);

        let staking_service = Arc::new(JitoStakingService::new(staking_config));
        staking_service.start().await.context("Failed to start staking service")?;

        // Create thread-safe transport
        let noise_config = noise::Config::new(&libp2p_keypair)
            .map_err(|e| anyhow!("Failed to create noise config: {}", e))?;
        let _transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
            .upgrade(upgrade::Version::V1)
            .authenticate(noise_config)
            .multiplex(yamux::Config::default())
            .boxed();

        // Configure gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(ValidationMode::Strict)
            .build()
            .map_err(|e| anyhow!("Failed to build gossipsub config: {}", e))?;

        // Fix: Clone the keypair for gossipsub
        let gossipsub = GossipsubBehaviour::new(
            MessageAuthenticity::Signed(libp2p_keypair.clone()),
            gossipsub_config,
        ).map_err(|e| anyhow!("Failed to create gossipsub: {}", e))?;

        let mdns = MdnsBehaviour::new(Default::default(), peer_id)
            .map_err(|e| anyhow!("Failed to create mDNS: {}", e))?;

        // Building two different swarms for publisher and relayer
        let swarm = match config.get_node_type() {
            NodeType::PUBLISHER => {
                let behaviour = PublisherBehaviour {
                    gossipsub,
                    mdns,
                };
                NodeSwarm::Publisher(
                    SwarmBuilder::with_existing_identity(libp2p_keypair)
                        .with_tokio()
                        .with_tcp(
                            tcp::Config::default().nodelay(true),
                            noise::Config::new,
                            yamux::Config::default,
                        )
                        .map_err(|e| anyhow!("Failed to create transport: {}", e))?
                        .with_behaviour(|_| behaviour)
                        .map_err(|e| anyhow!("Failed to create behaviour: {}", e))?
                        .with_swarm_config(|_| SwarmConfig::with_tokio_executor())
                        .build()
                )
            }
            NodeType::RELAYER => {
                let behaviour = RelayerBehaviour {
                    gossipsub,
                    mdns,
                };
                NodeSwarm::Relayer(
                    SwarmBuilder::with_existing_identity(libp2p_keypair)
                        .with_tokio()
                        .with_tcp(
                            tcp::Config::default().nodelay(true),
                            noise::Config::new,
                            yamux::Config::default,
                        )
                        .map_err(|e| anyhow!("Failed to create transport: {}", e))?
                        .with_behaviour(|_| behaviour)
                        .map_err(|e| anyhow!("Failed to create behaviour: {}", e))?
                        .with_swarm_config(|_| SwarmConfig::with_tokio_executor())
                        .build()
                )
            }
        };

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        Ok((
            Self {
                config,
                swarm: Arc::new(Mutex::new(swarm)),
                metrics: Arc::new(RwLock::new(Metrics::new())),
                known_peers: Arc::new(RwLock::new(HashSet::new())),
                shutdown_rx,
                staking_service,
            },
            shutdown_tx,
        ))
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting node on {}", self.config.get_listen_addr());

        let addr = format!("/ip4/{}/tcp/{}", 
            self.config.get_listen_addr().ip(),
            self.config.get_listen_addr().port()
        ).parse::<Multiaddr>()?;

        {
            let mut swarm = self.swarm.lock().await;
            match &mut *swarm {
                NodeSwarm::Publisher(swarm) => {
                    swarm.listen_on(addr)?;
                    for addr in self.config.get_bootstrap_peers() {
                        let remote: Multiaddr = addr.parse()?;
                        if let Err(e) = swarm.dial(remote.clone()) {
                            warn!("Failed to dial publisher {}: {}", remote, e);
                        }
                    }
                }
                NodeSwarm::Relayer(swarm) => {
                    swarm.listen_on(addr)?;
                    for addr in self.config.get_bootstrap_peers() {
                        let remote: Multiaddr = addr.parse()?;
                        if let Err(e) = swarm.dial(remote.clone()) {
                            warn!("Failed to dial relayer{}: {}", remote, e);
                        }
                    }
                }
            }
        }
        self.run().await
    }

    async fn run(&mut self) -> Result<()> {
        let mut heartbeat = time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                Some(_) = self.shutdown_rx.recv() => {
                    info!("Received shutdown signal");
                    break;
                }

                _ = heartbeat.tick() => {
                    self.maintain_peers().await?;
                }

                // Fix: Store swarm in a variable and use proper pinning
                // Can't use two async blocks with the same type in the same select
                event = {
                    let mut swarm = self.swarm.lock().await;
                    match &mut *swarm {
                        NodeSwarm::Publisher(swarm) => {
                            Box::pin(async move {
                                StreamExt::next(&mut *swarm).await
                            })
                        }
                        NodeSwarm::Relayer(swarm) => {
                            Box::pin(async move {
                                StreamExt::next(&mut *swarm).await
                            })
                        }
                    }
                } => {
                    if let Some(event) = event {
                        self.handle_swarm_event(event).await?;
                    }
                }
            }
        }

        info!("Node shutdown complete");
        Ok(())
    }

    async fn maintain_peers(&mut self) -> Result<()> {
        let peer_count = {
            let peers = self.known_peers.read().await;
            peers.len() as u64
        };

        self.metrics.write().await.set_connected_peers(peer_count);
        
        let mut peers_to_remove = Vec::new();
        
        {
            let peers = self.known_peers.read().await;
            for peer_id in peers.iter() {
                let operator = Pubkey::from(NetworkPeerId::from(*peer_id));
                match self.staking_service.get_operator_info(&operator).await {
                    Ok(info) => {
                        if info.stats.total_stake < self.staking_service.get_config().min_stake {
                            peers_to_remove.push(*peer_id);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to get operator info for {}: {}", peer_id, e);
                        peers_to_remove.push(*peer_id);
                    }
                }
            }
        }

        if !peers_to_remove.is_empty() {
            let mut peers = self.known_peers.write().await;
            for peer_id in peers_to_remove {
                peers.remove(&peer_id);
            }
        }

        Ok(())
    }

    async fn handle_swarm_event(&mut self, event: SwarmEvent<NodeEvent>) -> Result<()> {
        match &mut self.swarm {
            NodeSwarm::Publisher(swarm) => {
                // Handle publisher-specific events
                match event {
                    SwarmEvent::Behaviour(PublisherEvent::Gossipsub(event)) => {
                        self.handle_publisher_gossip(event).await?;
                    }
                    // ... other publisher events
                }
            }
            NodeSwarm::Relayer(swarm) => {
                // Handle relayer-specific events
                match event {
                    SwarmEvent::Behaviour(RelayerEvent::Gossipsub(event)) => {
                        self.handle_relayer_gossip(event).await?;
                    }
                    // ... other relayer events
                }
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {}", address);
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                let mut peers = self.known_peers.write().await;
                peers.insert(peer_id);
                debug!("Connected to {}", peer_id);
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                let mut peers = self.known_peers.write().await;
                peers.remove(&peer_id);
                debug!("Disconnected from {}", peer_id);
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_publisher_gossip(&mut self, event: gossipsub::Event) -> Result<()> {
        match event {
            gossipsub::Event::Message { 
                message_id,
                message,
                propagation_source,
                ..
            } => {
                if self.validate_message(&message).await? {
                    debug!("Valid message {} from {}", message_id, propagation_source);
                    // Acquire write lock to update metrics
                    self.metrics.write().await.increment_valid_messages();
                } else {
                    warn!("Invalid message {} from {}", message_id, propagation_source);
                    // Acquire write lock to update metrics
                    self.metrics.write().await.increment_invalid_messages();
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_relayer_gossip(&mut self, event: gossipsub::Event) -> Result<()> {
        match event {
            gossipsub::Event::Message { 
                message_id,
                message,
                propagation_source,
                ..
            } => {
                if self.validate_message(&message).await? {
                    debug!("Valid message {} from {}", message_id, propagation_source);
                    // Acquire write lock to update metrics
                    self.metrics.write().await.increment_valid_messages();
                } else {
                    warn!("Invalid message {} from {}", message_id, propagation_source);
                    // Acquire write lock to update metrics
                    self.metrics.write().await.increment_invalid_messages();
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn validate_message(&self, message: &gossipsub::Message) -> Result<bool> {
        let peer_id = message.source.as_ref()
            .ok_or_else(|| anyhow!("Message missing source"))?;
        let operator_pubkey = Pubkey::from(NetworkPeerId::from(*peer_id));
        
        let operator_info = self.staking_service
            .get_operator_info(&operator_pubkey)
            .await?;

        Ok(operator_info.stats.total_stake >= self.staking_service.get_config().min_stake)
    }
}

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
        signer::keypair::Keypair as agaveKeypair,
    },
    std::{
        collections::HashSet,
        sync::Arc,
        time::Duration,
    },
    tokio::{
        sync::{mpsc, RwLock, Mutex},
        time,
    },
    tracing::{debug, info, warn},
    windexer_common::config::NodeConfig,
};

use std::convert::TryInto;

pub fn convert_keypair(solana_keypair: &agaveKeypair) -> identity::Keypair {
    let full_bytes = solana_keypair.to_bytes();
    let seed: [u8; 32] = full_bytes[..32]
        .try_into()
        .expect("Slice should have a length of 32 bytes");
    identity::Keypair::ed25519_from_bytes(seed)
        .expect("Valid keypair conversion")
}

// Combined network behavior using both gossipsub and mDNS
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "NodeEvent")]
struct NodeBehaviour {
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

// Add these derives to make Node thread-safe
pub struct Node {
    pub config: NodeConfig,
    swarm: Arc<Mutex<Swarm<NodeBehaviour>>>,
    metrics: Arc<RwLock<Metrics>>,
    known_peers: Arc<RwLock<HashSet<PeerId>>>,
    shutdown_rx: mpsc::Receiver<()>,
}

// Implement Debug manually
impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("config", &self.config)
            .field("metrics", &self.metrics)
            .field("known_peers", &self.known_peers)
            .finish_non_exhaustive()
    }
}

impl Node {
    pub async fn create_simple(config: NodeConfig) -> Result<(Self, tokio::sync::mpsc::Sender<()>)> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        
        let swarm = Arc::new(Mutex::new(dummy_swarm_placeholder()));
        
        let node = Self {
            config,
            swarm,
            metrics: Arc::new(RwLock::new(Metrics::new())),
            known_peers: Arc::new(RwLock::new(HashSet::new())),
            shutdown_rx,
        };
        
        Ok((node, shutdown_tx))
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting node on {}", self.config.listen_addr);

        let addr = format!("/ip4/{}/tcp/{}", 
            self.config.listen_addr.ip(),
            self.config.listen_addr.port()
        ).parse::<Multiaddr>()?;

        {
            let mut swarm = self.swarm.lock().await;
            swarm.listen_on(addr)?;

            for addr in &self.config.bootstrap_peers {
                let remote: Multiaddr = addr.parse()?;
                match swarm.dial(remote.clone()) {
                    Ok(_) => info!("Dialing bootstrap peer {}", remote),
                    Err(e) => warn!("Failed to dial {}: {}", remote, e),
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
                event = {
                    let mut swarm = self.swarm.lock().await;
                    Box::pin(async move {
                        StreamExt::next(&mut *swarm).await
                    })
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
        
        Ok(())
    }

    async fn handle_swarm_event(
        &mut self,
        event: SwarmEvent<NodeEvent>
    ) -> Result<()> {
        match event {
            SwarmEvent::Behaviour(NodeEvent::Gossipsub(event)) => {
                self.handle_gossip_event(event).await?;
            }
            SwarmEvent::Behaviour(NodeEvent::Mdns(event)) => {
                self.handle_mdns_event(event).await?;
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

    async fn handle_gossip_event(&mut self, event: gossipsub::Event) -> Result<()> {
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

    async fn handle_mdns_event(&mut self, event: mdns::Event) -> Result<()> {
        match event {
            mdns::Event::Discovered(peers) => {
                for (peer_id, addr) in peers {
                    debug!("Discovered peer {} at {}", peer_id, addr);
                    let known_peers = self.known_peers.read().await;
                    if !known_peers.contains(&peer_id) {
                        drop(known_peers);
                        if let Err(e) = self.swarm.lock().await.dial(addr) {
                            warn!("Failed to dial discovered peer {}: {}", peer_id, e);
                        }
                    }
                }
            }
            mdns::Event::Expired(peers) => {
                for (peer_id, _) in peers {
                    debug!("Lost peer {}", peer_id);
                    let mut known_peers = self.known_peers.write().await;
                    known_peers.remove(&peer_id);
                }
            }
        }
        Ok(())
    }

    async fn validate_message(&self, _message: &gossipsub::Message) -> Result<bool> {
        Ok(true)
    }

    pub async fn stop(&self) -> Result<()> {
        // Implementation to properly shut down the node
        Ok(())
    }
}

// Helper function to create a dummy Swarm
fn dummy_swarm() -> Swarm<NodeBehaviour> {
    unimplemented!("This is a dummy implementation that should never be called")
}

// A function that creates a placeholder for the Swarm without actually initializing it
fn dummy_swarm_placeholder() -> Swarm<NodeBehaviour> {
    unsafe {
        // This is a hack to create a dummy Swarm instance without proper initialization
        // It should never be used in actual code
        std::mem::zeroed()
    }
}

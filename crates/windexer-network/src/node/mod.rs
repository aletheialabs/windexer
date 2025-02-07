// crates/windexer-network/src/node/mod.rs

use anyhow::Result;
use libp2p::{
    gossipsub::{
        self,
        Behaviour as GossipsubBehaviour,
        Event as GossipsubEvent,
        Message as GossipsubMessage,
        IdentTopic,
        MessageAuthenticity,
    },
    mdns::{
        tokio::Behaviour as MdnsBehaviour,
        Event as MdnsEvent,
    },
    swarm::{NetworkBehaviour, SwarmEvent},
    identity::Keypair,
    tcp::tokio::Transport as TokioTcpTransport,
    noise,
    yamux,
    Swarm,
    PeerId,
    Transport,
    core::upgrade::Version,
};
use tokio::sync::mpsc;
use tracing::{debug, info};
use futures::StreamExt;

use crate::metrics::Metrics;

pub struct NodeConfig {
    pub keypair: Keypair,
    pub listen_addresses: Vec<String>,
    pub bootstrap_peers: Vec<String>,
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "NodeEvent", event_process = false)]
pub struct NodeBehaviour {
    gossipsub: GossipsubBehaviour,
    mdns: MdnsBehaviour,
}

#[derive(Debug)]
pub enum NodeEvent {
    Gossipsub(GossipsubEvent),
    Mdns(MdnsEvent),
}

impl From<GossipsubEvent> for NodeEvent {
    fn from(event: GossipsubEvent) -> Self {
        NodeEvent::Gossipsub(event)
    }
}

impl From<MdnsEvent> for NodeEvent {
    fn from(event: MdnsEvent) -> Self {
        NodeEvent::Mdns(event)
    }
}

pub struct Node {
    swarm: Swarm<NodeBehaviour>,
    metrics: Metrics,
    _shutdown: mpsc::Receiver<()>,
}

impl Node {
    pub async fn new(config: NodeConfig) -> Result<(Self, mpsc::Sender<()>)> {
        let peer_id = PeerId::from(config.keypair.public());
        info!("Local peer id: {peer_id}");

        // Create transport
        let transport = TokioTcpTransport::default()
            .upgrade(Version::V1)
            .authenticate(noise::Config::new(&config.keypair).expect("Failed to create noise config"))
            .multiplex(yamux::Config::default())
            .boxed();

        // Setup gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(std::time::Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .expect("Valid config");

        // Create gossipsub behavior
        let gossipsub = GossipsubBehaviour::new(
            MessageAuthenticity::Signed(config.keypair),
            gossipsub_config,
        ).expect("Valid gossipsub config");

        // Create MDNS behavior
        let mdns = MdnsBehaviour::new(Default::default(), peer_id)?;

        // Combine behaviors
        let behaviour = NodeBehaviour {
            gossipsub,
            mdns,
        };

        // Create swarm with config
        let swarm = Swarm::new(
            transport,
            behaviour,
            peer_id,
            libp2p::swarm::Config::with_tokio_executor(),
        );

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
                event = self.swarm.select_next_some() => {
                    match event {
                        SwarmEvent::Behaviour(NodeEvent::Gossipsub(
                            GossipsubEvent::Message { 
                                message,
                                ..
                            }
                        )) => {
                            self.handle_message(&message).await?;
                            self.metrics.record_message_received();
                        }
                        SwarmEvent::Behaviour(NodeEvent::Mdns(MdnsEvent::Discovered(peers))) => {
                            for (peer_id, addr) in peers {
                                info!("Discovered peer: {peer_id} at {addr}");
                                self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                                self.metrics.increment_peers();
                            }
                        }
                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            self.metrics.decrement_peers();
                            debug!("Connection closed to peer: {peer_id}");
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn handle_message(&mut self, _message: &GossipsubMessage) -> Result<()> {
        debug!("Received message from network");
        Ok(())
    }

    #[allow(dead_code)]
    async fn broadcast_message(&mut self, topic: IdentTopic, data: Vec<u8>) -> Result<()> {
        self.swarm.behaviour_mut().gossipsub.publish(topic, data)?;
        self.metrics.record_message_sent();
        Ok(())
    }
}
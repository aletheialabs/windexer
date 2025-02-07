// crates/windexer-network/src/node/mod.rs

use {
    crate::{metrics::Metrics, NetworkPeerId}, 
    anyhow::{anyhow, Context, Result}, 
    libp2p::{
        core::upgrade, 
        gossipsub::{self, Behaviour as GossipsubBehaviour, Message as GossipsubMessage, MessageAuthenticity}, 
        mdns::{self, tokio::Behaviour as MdnsBehaviour}, 
        noise, 
        swarm::{NetworkBehaviour, Swarm}, 
        tcp, 
        yamux, 
        PeerId, 
        Transport
    }, 
    solana_sdk::{
        pubkey::Pubkey, 
        signer::keypair::Keypair as SolanaKeypair,
    }, 
    std::{sync::Arc, time::Duration}, 
    tokio::sync::mpsc, 
    windexer_jito_staking::{JitoStakingService, StakingConfig}
};

pub fn convert_keypair(solana_keypair: &SolanaKeypair) -> libp2p::identity::Keypair {
    let secret = solana_keypair.to_bytes();
    libp2p::identity::Keypair::ed25519_from_bytes(secret).expect("Valid keypair conversion")
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "NodeEvent")]
pub struct NodeBehaviour {
    gossipsub: GossipsubBehaviour,
    mdns: MdnsBehaviour,
}

#[derive(Debug)]
pub enum NodeEvent {
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

pub struct NodeConfig {
    pub keypair: SolanaKeypair,
    pub staking_config: StakingConfig,
}

pub struct Node {
    swarm: Swarm<NodeBehaviour>,
    metrics: Metrics,
    _shutdown: mpsc::Receiver<()>,
    staking_service: Arc<JitoStakingService>,
}

impl Node {
    pub async fn new(config: NodeConfig) -> Result<(Self, mpsc::Sender<()>)> {
        let libp2p_keypair = convert_keypair(&config.keypair);
        let peer_id = PeerId::from(libp2p_keypair.public());
        
        // Initialize staking service first
        let staking_service = Arc::new(JitoStakingService::new(config.staking_config));
        staking_service.start().await.with_context(|| "Failed to start staking service")?;

        // Create transport
        let noise_config = noise::Config::new(&libp2p_keypair)
            .with_context(|| "Failed to create noise config")?;
            
        let transport = tcp::tokio::Transport::new(tcp::Config::default())
            .upgrade(upgrade::Version::V1)
            .authenticate(noise_config)
            .multiplex(yamux::Config::default())
            .boxed();

        // Configure gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .with_context(|| "Failed to build gossipsub config")?;

        let gossipsub = GossipsubBehaviour::new(
            MessageAuthenticity::Signed(libp2p_keypair),
            gossipsub_config,
        ).map_err(|e| anyhow!("Failed to create gossipsub behavior: {}", e))?;

        let mdns = MdnsBehaviour::new(Default::default(), peer_id)
            .map_err(|e| anyhow!("Failed to create mDNS behavior: {}", e))?;

        let behaviour = NodeBehaviour {
            gossipsub,
            mdns,
        };

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
                staking_service,
            },
            shutdown_tx,
        ))
    }

    fn create_peer_score_params() -> gossipsub::PeerScoreParams {
        gossipsub::PeerScoreParams {
            topics: Default::default(),
            topic_score_cap: 1.0,
            ip_colocation_factor_weight: 0.0,
            ip_colocation_factor_threshold: 1.0,
            behaviour_penalty_weight: -1.0,
            behaviour_penalty_threshold: 0.0,
            behaviour_penalty_decay: 0.9,
            decay_interval: Duration::from_secs(1),
            decay_to_zero: 0.01,
            retain_score: Duration::from_secs(3600),
            ..Default::default()
        }
    }

    async fn validate_message(&self, message: &GossipsubMessage) -> Result<bool> {
        let peer_id = message.source.as_ref().ok_or_else(|| anyhow!("Message missing source"))?;
        let operator_pubkey = Pubkey::from(NetworkPeerId::from(*peer_id));
        let operator_info = self.staking_service
            .get_operator_info(&operator_pubkey)
            .await?;

        Ok(operator_info.stats.total_stake >= self.staking_service.get_config().min_stake)
    }
}
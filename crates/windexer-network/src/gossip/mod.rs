// crates/windexer-network/src/gossip/mod.rs

use {
    std::sync::Arc,
    anyhow::Result,
    libp2p::{gossipsub::TopicHash, PeerId},
    serde::{Deserialize, Serialize},
    tokio::sync::RwLock,
    tracing::debug,
    solana_sdk::pubkey::Pubkey,
    windexer_jito_staking::{JitoStakingService, OperatorInfo},
    crate::NetworkPeerId,
};

mod mesh_manager;
mod message_handler;
mod topic_handler;

pub use mesh_manager::MeshManager;
pub use message_handler::MessageHandler;
pub use topic_handler::TopicHandler;

/// Main gossip subsystem that coordinates network message propagation
/// with stake-weighted validation and peer scoring
pub struct GossipSubsystem {
    mesh_manager: Arc<RwLock<MeshManager>>,
    message_handler: Arc<RwLock<MessageHandler>>,
    topic_handler: Arc<RwLock<TopicHandler>>,
    staking_service: Arc<JitoStakingService>,
    config: GossipConfig,
}

impl GossipSubsystem {
    pub fn new(
        config: GossipConfig,
        staking_service: Arc<JitoStakingService>
    ) -> Self {
        let mesh_manager = Arc::new(RwLock::new(MeshManager::new(config.clone())));
        let message_handler = Arc::new(RwLock::new(MessageHandler::new(1000)));
        let topic_handler = Arc::new(RwLock::new(TopicHandler::new(config.clone())));
        
        Self {
            mesh_manager,
            message_handler,
            topic_handler,
            staking_service,
            config,
        }
    }

    pub async fn handle_message(&self, message: GossipMessage) -> Result<()> {
        let operator_pubkey = Pubkey::from(NetworkPeerId::from(message.source));
        let operator_info = self.staking_service
            .get_operator_info(&operator_pubkey)
            .await?;

        if !self.has_sufficient_stake(&operator_info).await? {
            debug!("Ignoring message from peer with insufficient stake");
            return Ok(());
        }

        let mut message_handler = self.message_handler.write().await;
        let topic_handler = self.topic_handler.write().await;

        message_handler.handle_message(
            message.source,
            message.clone(),
            &self.staking_service
        ).await?;

        for topic_str in &message.topics {
            let topic = TopicHash::from_raw(topic_str);
            topic_handler.publish(&topic, message.clone()).await?;
        }

        Ok(())
    }

    pub async fn subscribe(&self, topic: TopicHash) -> Result<()> {
        let mut mesh_manager = self.mesh_manager.write().await;
        let mut topic_handler = self.topic_handler.write().await;

        let peers = self.select_mesh_peers(&topic).await?;
        for peer in peers {
            mesh_manager.add_peer_to_mesh(peer, topic.clone())?;
        }

        topic_handler.subscribe(topic);
        Ok(())
    }

    async fn has_sufficient_stake(&self, info: &OperatorInfo) -> Result<bool> {
        Ok(info.stats.total_stake >= self.staking_service.get_config().min_stake)
    }

    async fn select_mesh_peers(&self, topic: &TopicHash) -> Result<Vec<PeerId>> {
        let mesh_manager = self.mesh_manager.read().await;
        let current_peers = mesh_manager.get_mesh_peers(topic);

        let mut peer_stakes = Vec::new();
        for peer in current_peers {
            let operator_pubkey = Pubkey::from(NetworkPeerId::from(peer));
            if let Ok(info) = self.staking_service.get_operator_info(&operator_pubkey).await {
                peer_stakes.push((peer, info.stats.total_stake));
            }
        }

        peer_stakes.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(peer_stakes.into_iter()
            .take(self.config.mesh_n)
            .map(|(peer, _)| peer)
            .collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipConfig {
    pub heartbeat_interval: std::time::Duration,
    pub mesh_n: usize,
    pub mesh_n_low: usize,
    pub mesh_n_high: usize,
    pub gossip_factor: f64,
    
    pub min_peer_stake: u64,
    pub target_stake_per_topic: u64,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: std::time::Duration::from_secs(1),
            mesh_n: 6,
            mesh_n_low: 4,
            mesh_n_high: 12,
            gossip_factor: 0.25,
            min_peer_stake: 1_000_000_000, // 1 SOL
            target_stake_per_topic: 100_000_000_000, // 100 SOL
        }
    }
}

#[derive(Debug, Clone)]
pub struct GossipMessage {
    pub source: PeerId,
    pub topics: Vec<String>,
    pub payload: Vec<u8>,
    pub message_id: Vec<u8>,
    pub timestamp: i64,
}

#[derive(Debug)]
pub enum GossipEvent {
    MessageReceived {
        from: PeerId,
        message: GossipMessage,
    }
}

impl Serialize for GossipMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("GossipMessage", 5)?;
        state.serialize_field("source", &self.source.to_string())?;
        state.serialize_field("topics", &self.topics.iter().map(|t| t.to_string()).collect::<Vec<_>>())?;
        state.serialize_field("payload", &self.payload)?;
        state.serialize_field("message_id", &self.message_id)?;
        state.serialize_field("timestamp", &self.timestamp)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for GossipMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field { Source, Topics, Payload, MessageId, Timestamp }

        struct GossipMessageVisitor;

        impl<'de> Visitor<'de> for GossipMessageVisitor {
            type Value = GossipMessage;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct GossipMessage")
            }

            fn visit_map<V>(self, mut map: V) -> Result<GossipMessage, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut source = None;
                let mut topics = None;
                let mut payload = None;
                let mut message_id = None;
                let mut timestamp = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Source => {
                            let s: String = map.next_value()?;
                            source = Some(s.parse().map_err(de::Error::custom)?);
                        }
                        Field::Topics => {
                            let t: Vec<String> = map.next_value()?;
                            topics = Some(t);
                        }
                        Field::Payload => payload = Some(map.next_value()?),
                        Field::MessageId => message_id = Some(map.next_value()?),
                        Field::Timestamp => timestamp = Some(map.next_value()?),
                    }
                }

                Ok(GossipMessage {
                    source: source.ok_or_else(|| de::Error::missing_field("source"))?,
                    topics: topics.ok_or_else(|| de::Error::missing_field("topics"))?,
                    payload: payload.ok_or_else(|| de::Error::missing_field("payload"))?,
                    message_id: message_id.ok_or_else(|| de::Error::missing_field("message_id"))?,
                    timestamp: timestamp.ok_or_else(|| de::Error::missing_field("timestamp"))?,
                })
            }
        }

        const FIELDS: &[&str] = &["source", "topics", "payload", "message_id", "timestamp"];
        deserializer.deserialize_struct("GossipMessage", FIELDS, GossipMessageVisitor)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    BlockData,
    AccountUpdate,
    Transaction,
    ConsensusVote,
    PeerAnnouncement,
    HeartBeat,
}
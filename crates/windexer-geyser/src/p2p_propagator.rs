// crates/windexer-geyser/src/p2p_propagator.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    libp2p::gossipsub::{Message, MessageId},
    solana_sdk::{clock::Slot, pubkey::Pubkey},
    windexer_common::{
        config::network::GossipConfig,
        crypto::{BLSKeypair, Ed25519Keypair},
        errors::PropagationError,
        types::{TideBatch, TideHeader},
        utils::time::timestamp,
    },
    windexer_network::{
        gossip::{GossipManager, MessagePriority},
        metrics::NetworkMetrics,
    },
    std::{
        collections::{BTreeMap, VecDeque},
        sync::{Arc, Mutex},
    },
    tokio::sync::mpsc,
};

const TIDE_PROTOCOL_VERSION: u8 = 0x01;

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub enum TidePriority {
    High,
    Medium,
    Low,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct TideMessage {
    pub version: u8,
    pub slot: Slot,
    pub batch: TideBatch,
    pub merkle_root: [u8; 32],
    pub bls_signature: Vec<u8>,
    pub ed25519_signature: Vec<u8>,
    pub priority: TidePriority,
}

impl TideMessage {
    pub fn new(
        batch: TideBatch,
        merkle_root: [u8; 32],
        bls_keypair: &BLSKeypair,
        ed25519_keypair: &Ed25519Keypair,
        priority: TidePriority,
    ) -> Result<Self, PropagationError> {
        let mut msg = Self {
            version: TIDE_PROTOCOL_VERSION,
            slot: batch.slot,
            batch,
            merkle_root,
            bls_signature: Vec::new(),
            ed25519_signature: Vec::new(),
            priority,
        };

        msg.bls_signature = bls_keypair.sign(&msg.serialize()?)?;
        
        msg.ed25519_signature = ed25519_keypair.sign(&msg.serialize()?)?;

        Ok(msg)
    }

    pub fn serialize(&self) -> Result<Vec<u8>, PropagationError> {
        self.try_to_vec()
            .map_err(|e| PropagationError::SerializationError(e.to_string()))
    }

    pub fn verify(&self, bls_pubkey: &[u8], ed25519_pubkey: &Pubkey) -> Result<(), PropagationError> {
        BLSKeypair::verify(
            bls_pubkey,
            &self.serialize()?,
            &self.bls_signature
        )?;

        Ed25519Keypair::verify(
            ed25519_pubkey,
            &self.serialize()?,
            &self.ed25519_signature
        )?;

        Ok(())
    }
}

struct PriorityQueue {
    high: VecDeque<TideMessage>,
    medium: VecDeque<TideMessage>,
    low: VecDeque<TideMessage>,
}

impl PriorityQueue {
    fn new() -> Self {
        Self {
            high: VecDeque::with_capacity(1000),
            medium: VecDeque::with_capacity(5000),
            low: VecDeque::with_capacity(10000),
        }
    }

    fn push(&mut self, msg: TideMessage) {
        match msg.priority {
            TidePriority::High => self.high.push_back(msg),
            TidePriority::Medium => self.medium.push_back(msg),
            TidePriority::Low => self.low.push_back(msg),
        }
    }

    fn pop(&mut self) -> Option<TideMessage> {
        self.high.pop_front()
            .or_else(|| self.medium.pop_front())
            .or_else(|| self.low.pop_front())
    }
}

pub struct TidePropagator {
    gossip: Arc<GossipManager>,
    topic: String,
    queue: Mutex<PriorityQueue>,
    metrics: Arc<NetworkMetrics>,
    sender: mpsc::Sender<TideMessage>,
}

impl TidePropagator {
    pub fn new(
        gossip: Arc<GossipManager>,
        config: &GossipConfig,
        metrics: Arc<NetworkMetrics>,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel(10000);

        let gossip_clone = gossip.clone();
        let metrics_clone = metrics.clone();
        let topic = config.tide_topic.clone();
        
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let message = Message {
                    source: None,
                    data: msg.serialize().expect("Failed to serialize Tide message"),
                    sequence_number: None,
                    topic: topic.clone().into(),
                };

                match gossip_clone.publish(message).await {
                    Ok(message_id) => {
                        metrics_clone.increment_published_messages();
                        tracing::debug!("Published Tide message: {:?}", message_id);
                    }
                    Err(e) => {
                        metrics_clone.increment_publish_errors();
                        tracing::error!("Failed to publish Tide message: {}", e);
                    }
                }
            }
        });

        Self {
            gossip,
            topic: config.tide_topic.clone(),
            queue: Mutex::new(PriorityQueue::new()),
            metrics,
            sender: tx,
        }
    }

    pub async fn propagate(&self, msg: TideMessage) -> Result<MessageId, PropagationError> {
        let serialized = msg.serialize()?;
        self.metrics.record_message_size(serialized.len());

        let message = Message {
            source: None,
            data: serialized,
            sequence_number: None,
            topic: self.topic.clone().into(),
        };

        self.sender.send(msg).await.map_err(|e| {
            PropagationError::ChannelError(e.to_string())
        })?;

        Ok(MessageId::from(timestamp().to_be_bytes()))
    }

    pub fn handle_message(&self, msg: &Message) -> Result<(), PropagationError> {
        let tide_msg: TideMessage = BorshDeserialize::try_from_slice(&msg.data)
            .map_err(|e| PropagationError::DeserializationError(e.to_string()))?;

        if tide_msg.version != TIDE_PROTOCOL_VERSION {
            return Err(PropagationError::VersionMismatch);
        }

        tide_msg.verify(
            &self.gossip.bls_public_key(),
            &self.gossip.ed25519_public_key()
        )?;

        let mut queue = self.queue.lock().unwrap();
        queue.push(tide_msg);

        self.metrics.increment_received_messages();
        Ok(())
    }

    pub fn next_message(&self) -> Option<TideMessage> {
        self.queue.lock().unwrap().pop()
    }

    pub fn metrics(&self) -> BTreeMap<String, u64> {
        let mut map = BTreeMap::new();
        let queue = self.queue.lock().unwrap();
        
        map.insert("high_priority".into(), queue.high.len() as u64);
        map.insert("medium_priority".into(), queue.medium.len() as u64);
        map.insert("low_priority".into(), queue.low.len() as u64);

        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windexer_common::crypto::{generate_bls_keypair, generate_ed25519_keypair};

    #[test]
    fn test_tide_message_roundtrip() {
        let bls_keypair = generate_bls_keypair();
        let ed25519_keypair = generate_ed25519_keypair();
        let batch = TideBatch {
            slot: 42,
            data: vec![1, 2, 3],
            timestamp: timestamp(),
        };

        let original = TideMessage::new(
            batch,
            [0u8; 32],
            &bls_keypair,
            &ed25519_keypair,
            TidePriority::High
        ).unwrap();

        let serialized = original.serialize().unwrap();
        let deserialized: TideMessage = BorshDeserialize::try_from_slice(&serialized).unwrap();

        assert_eq!(original.version, deserialized.version);
        assert_eq!(original.slot, deserialized.slot);
        assert_eq!(original.priority, deserialized.priority);
    }

    #[tokio::test]
    async fn test_propagation_flow() {
        let config = GossipConfig::default();
        let metrics = Arc::new(NetworkMetrics::new());
        let gossip = Arc::new(GossipManager::new(config.clone(), metrics.clone()));
        let propagator = TidePropagator::new(gossip, &config, metrics);

        let bls_keypair = generate_bls_keypair();
        let ed25519_keypair = generate_ed25519_keypair();
        let batch = TideBatch {
            slot: 1,
            data: vec![4, 5, 6],
            timestamp: timestamp(),
        };

        let msg = TideMessage::new(
            batch,
            [1u8; 32],
            &bls_keypair,
            &ed25519_keypair,
            TidePriority::Medium
        ).unwrap();

        propagator.propagate(msg).await.unwrap();
        assert_eq!(propagator.metrics()["medium_priority"], 1);
    }
}

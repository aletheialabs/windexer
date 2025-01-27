use super::{GossipEvent, GossipMessage};
use anyhow::Result;
use libp2p::{PeerId, gossipsub::TopicHash};
use std::collections::{HashMap, HashSet, VecDeque};
use tokio::sync::mpsc;
use tracing::{debug, warn};

pub struct MessageHandler {
    message_cache: VecDeque<MessageCacheEntry>,
    seen_messages: HashSet<Vec<u8>>,
    pending_messages: HashMap<TopicHash, Vec<GossipMessage>>,
    max_cache_size: usize,
    event_tx: mpsc::Sender<GossipEvent>,
    event_rx: mpsc::Receiver<GossipEvent>,
}

struct MessageCacheEntry {
    message_id: Vec<u8>,
    topics: Vec<TopicHash>,
    expiry: std::time::Instant,
}

impl MessageHandler {
    pub fn new(max_cache_size: usize) -> Self {
        let (event_tx, event_rx) = mpsc::channel(1000);
        Self {
            message_cache: VecDeque::with_capacity(max_cache_size),
            seen_messages: HashSet::new(),
            pending_messages: HashMap::new(),
            max_cache_size,
            event_tx,
            event_rx,
        }
    }

    pub async fn handle_message(&mut self, from: PeerId, message: GossipMessage) -> Result<()> {
        let message_id = message.id.clone();
        
        if self.seen_messages.contains(&message_id) {
            debug!("Ignoring already seen message: {:?}", message_id);
            return Ok(());
        }

        self.cache_message(message_id.clone());
        
        for topic in &message.topics {
            self.pending_messages
                .entry(topic.clone())
                .or_default()
                .push(message.clone());
        }

        self.event_tx
            .send(GossipEvent::MessageReceived { from, message })
            .await?;

        Ok(())
    }

    fn cache_message(&mut self, message_id: Vec<u8>) {
        if self.message_cache.len() >= self.max_cache_size {
            if let Some(old) = self.message_cache.pop_front() {
                self.seen_messages.remove(&old.message_id);
            }
        }

        let entry = MessageCacheEntry {
            message_id: message_id.clone(),
            topics: Vec::new(),
            expiry: std::time::Instant::now() + std::time::Duration::from_secs(60),
        };

        self.message_cache.push_back(entry);
        self.seen_messages.insert(message_id);
    }

    pub fn get_pending_messages(&mut self, topic: &TopicHash) -> Vec<GossipMessage> {
        self.pending_messages.remove(topic).unwrap_or_default()
    }

    pub async fn next_event(&mut self) -> Option<GossipEvent> {
        self.event_rx.recv().await
    }

    pub fn cleanup_expired(&mut self) {
        let now = std::time::Instant::now();
        while let Some(entry) = self.message_cache.front() {
            if entry.expiry > now {
                break;
            }
            if let Some(expired) = self.message_cache.pop_front() {
                self.seen_messages.remove(&expired.message_id);
            }
        }
    }
}
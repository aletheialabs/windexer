use super::{GossipConfig, GossipMessage};
use anyhow::Result;
use libp2p::gossipsub::TopicHash;
use std::collections::{HashMap, HashSet};
use tokio::sync::broadcast;

pub struct TopicHandler {
    topics: HashSet<TopicHash>,
    subscribers: HashMap<TopicHash, broadcast::Sender<GossipMessage>>,
    config: GossipConfig,
}

impl TopicHandler {
    pub fn new(config: GossipConfig) -> Self {
        Self {
            topics: HashSet::new(),
            subscribers: HashMap::new(),
            config,
        }
    }

    pub fn subscribe(&mut self, topic: TopicHash) -> broadcast::Receiver<GossipMessage> {
        self.topics.insert(topic.clone());
        
        let (tx, rx) = if let Some(tx) = self.subscribers.get(&topic) {
            (tx.clone(), tx.subscribe())
        } else {
            let (tx, rx) = broadcast::channel(1000);
            self.subscribers.insert(topic, tx.clone());
            (tx, rx)
        };

        rx
    }

    pub fn unsubscribe(&mut self, topic: &TopicHash) {
        self.topics.remove(topic);
        self.subscribers.remove(topic);
    }

    pub async fn publish(&self, topic: &TopicHash, message: GossipMessage) -> Result<()> {
        if let Some(tx) = self.subscribers.get(topic) {
            tx.send(message)?;
        }
        Ok(())
    }

    pub fn is_subscribed(&self, topic: &TopicHash) -> bool {
        self.topics.contains(topic)
    }

    pub fn get_topics(&self) -> &HashSet<TopicHash> {
        &self.topics
    }
}
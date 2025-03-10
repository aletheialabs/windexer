// crates/windexer-network/src/gossip/message_handler.rs

use {
    std::collections::{HashSet, VecDeque},
    libp2p::PeerId,
    tokio::sync::mpsc,
    tracing::debug,
    solana_sdk::pubkey::Pubkey,
    windexer_jito_staking::JitoStakingService,
    crate::{
        gossip::{GossipMessage, GossipEvent},
        NetworkPeerId,
    },
    anyhow::Result,
};

#[derive(Debug, Clone)]
pub struct MessageCacheEntry {
    pub message_id: Vec<u8>,
    pub topics: Vec<String>,
    pub expiry: std::time::Instant,
    pub priority: u8,
}

pub struct MessageHandler {
    seen_messages: HashSet<Vec<u8>>,
    message_cache: VecDeque<MessageCacheEntry>,
    max_cache_size: usize,
    event_tx: mpsc::Sender<GossipEvent>,
}

impl MessageHandler {
    pub fn new(max_cache_size: usize) -> Self {
        Self {
            seen_messages: HashSet::new(),
            message_cache: VecDeque::new(),
            max_cache_size,
            event_tx: mpsc::channel(100).0,
        }
    }

    pub async fn handle_message(
        &mut self,
        from: PeerId,
        message: GossipMessage,
        staking_service: &JitoStakingService,
    ) -> Result<()> {
        let operator_pubkey = Pubkey::from(NetworkPeerId::from(from));
        let operator_info = staking_service.get_operator_info(&operator_pubkey).await?;
        
        if operator_info.stats.total_stake < staking_service.get_config().min_stake {
            debug!("Ignoring message from peer with insufficient stake");
            return Ok(());
        }

        let message_id = message.message_id.clone();
        if self.seen_messages.contains(&message_id) {
            debug!("Ignoring already seen message: {:?}", message_id);
            return Ok(());
        }

        let priority = (operator_info.stats.total_stake as f64).log10() as u8;
        self.cache_message(message_id.clone(), priority);

        self.event_tx
            .send(GossipEvent::MessageReceived { from, message })
            .await?;

        Ok(())
    }

    fn cache_message(&mut self, message_id: Vec<u8>, priority: u8) {
        if self.message_cache.len() >= self.max_cache_size {
            self.prune_cache();
        }

        let entry = MessageCacheEntry {
            message_id: message_id.clone(),
            topics: Vec::new(),
            expiry: std::time::Instant::now() + std::time::Duration::from_secs(60),
            priority,
        };

        self.message_cache.push_back(entry);
        self.seen_messages.insert(message_id);
    }
    fn set_max_cache_size(&mut self, max_cache_size: usize) {
        self.max_cache_size = max_cache_size;
        debug!("Setting max cache");
        self.prune_cache();
    }

    fn get_max_cache_size(&self) -> usize {
        self.max_cache_size
    }

    fn get_cache_size(&self) -> usize {
        self.message_cache.len()
    }

    fn get_cache_entries(&self) -> Vec<MessageCacheEntry> {
        self.message_cache.iter().cloned().collect()
    }


    fn prune_cache(&mut self) {
        let now = std::time::Instant::now();
        self.message_cache.retain(|entry| entry.expiry > now);
        while self.message_cache.len() >= self.max_cache_size {
            self.message_cache.pop_front();
        }
    }
}
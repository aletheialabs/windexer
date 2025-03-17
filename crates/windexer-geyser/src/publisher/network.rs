// crates/windexer-geyser/src/publisher/network.rs

//! Network data publisher
//!
//! This module contains the implementation of a publisher that sends data to the
//! wIndexer network using libp2p gossipsub.

use {
    crate::{
        metrics::Metrics,
        publisher::{Publisher, PublisherConfig},
        ShutdownFlag,
    },
    anyhow::Result,
    std::{
        sync::{
            Arc,
            atomic::Ordering,
        },
    },
    windexer_common::{
        types::{
            account::AccountData,
            transaction::TransactionData,
            block::BlockData,
            block::EntryData,
        },
        crypto::SerializableKeypair,
        config::NodeConfig,
    },
    windexer_network::{
        node::Node,
    },
    solana_sdk::signer::keypair::Keypair,
    log::{error, warn},
    serde::{Deserialize, Serialize},
    tokio,
};

const ACCOUNT_TOPIC: &str = "windexer.accounts";
const TRANSACTION_TOPIC: &str = "windexer.transactions";
const BLOCK_TOPIC: &str = "windexer.blocks";
const ENTRY_TOPIC: &str = "windexer.entries";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NetworkMessage<T> {
    pub validator_id: Option<String>,
    pub data: T,
}

#[derive(Clone)]
pub struct NetworkPublisher {
    network_node_and_shutdown: Arc<(Node, tokio::sync::mpsc::Sender<()>)>,
    batch_size: usize,
    metrics: Arc<Metrics>,
    validator_id: Option<String>,
    shutdown: Arc<ShutdownFlag>,
}

impl NetworkPublisher {
    pub async fn new(config: PublisherConfig, shutdown: Arc<ShutdownFlag>) -> Result<Self> {
        let data_dir = std::env::temp_dir().join("windexer-geyser-plugin");
        std::fs::create_dir_all(&data_dir)?;
        
        let keypair = Keypair::new();
        
        let node_config = NodeConfig {
            listen_addr: config.network_addr.clone().parse().unwrap_or_else(|_| "127.0.0.1:8900".parse().unwrap()),
            bootstrap_peers: config.network_bootstrap_peers.clone(),
            metrics_addr: None,
            node_id: "windexer-geyser-plugin".to_string(),
            rpc_addr: "127.0.0.1:8899".parse().unwrap(),
            data_dir: data_dir.to_string_lossy().to_string(),
            solana_rpc_url: config.solana_rpc_url.clone().unwrap_or_else(|| "http://localhost:8899".to_string()),
            geyser_plugin_config: Some("{}".to_string()),
            keypair: SerializableKeypair::new(&keypair),
        };
        
        let (mock_node, shutdown_sender) = create_mock_node(node_config).await?;
        
        let network_node_and_shutdown = Arc::new((mock_node, shutdown_sender));
        
        let publisher = Self {
            network_node_and_shutdown,
            batch_size: config.batch_size,
            metrics: config.metrics.clone(),
            validator_id: config.validator_id.clone(),
            shutdown,
        };
        
        publisher.init_topics().await?;
        
        Ok(publisher)
    }
    
    async fn init_topics(&self) -> Result<()> {
        let _node = &self.network_node_and_shutdown.0;
        
        // MOCK IMPLEMENTATION: Currently we don't have the correct method names
        warn!("Network subscription methods have been mocked. No actual subscription is taking place.");
        
        Ok(())
    }
    
    async fn publish_message<T: Serialize>(&self, topic: &str, message: T) -> Result<()> {
        let network_message = NetworkMessage {
            validator_id: self.validator_id.clone(),
            data: message,
        };
        
        let _payload = serde_json::to_vec(&network_message)?;
        
        let _node = &self.network_node_and_shutdown.0;
        
        // MOCK IMPLEMENTATION: Currently we don't have the correct method names
        warn!("Network publish method has been mocked. Message to topic '{}' is not actually being published.", topic);
        
        Ok(())
    }
    
    fn batch_data<T>(data: &[T], batch_size: usize) -> Vec<Vec<T>>
    where
        T: Clone,
    {
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();
        
        for item in data {
            current_batch.push(item.clone());
            
            if current_batch.len() >= batch_size {
                batches.push(std::mem::take(&mut current_batch));
            }
        }
        
        if !current_batch.is_empty() {
            batches.push(current_batch);
        }
        
        batches
    }
}

impl Publisher for NetworkPublisher {
    fn publish_accounts(&self, accounts: &[AccountData]) -> Result<()> {
        if accounts.is_empty() {
            return Ok(());
        }
        
        let batches = Self::batch_data(accounts, self.batch_size);
        let batches_count = batches.len() as u64;
        
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        
        for batch in batches {
            let result = runtime.block_on(
                self.publish_message(ACCOUNT_TOPIC, batch)
            );
            
            if let Err(e) = result {
                self.metrics.account_publish_errors.fetch_add(1, Ordering::Relaxed);
                error!("Failed to publish account batch: {}", e);
                return Err(e);
            }
        }
        
        self.metrics.account_batches_published.fetch_add(batches_count, Ordering::Relaxed);
        Ok(())
    }
    
    fn publish_transactions(&self, transactions: &[TransactionData]) -> Result<()> {
        if transactions.is_empty() {
            return Ok(());
        }
        
        let batches = Self::batch_data(transactions, self.batch_size);
        let batches_count = batches.len() as u64;
        
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        
        for batch in batches {
            let result = runtime.block_on(
                self.publish_message(TRANSACTION_TOPIC, batch)
            );
            
            if let Err(e) = result {
                self.metrics.transaction_publish_errors.fetch_add(1, Ordering::Relaxed);
                error!("Failed to publish transaction batch: {}", e);
                return Err(e);
            }
        }
        
        self.metrics.transaction_batches_published.fetch_add(batches_count, Ordering::Relaxed);
        Ok(())
    }
    
    fn publish_block(&self, block: BlockData) -> Result<()> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        
        let result = runtime.block_on(
            self.publish_message(BLOCK_TOPIC, block)
        );
        
        if let Err(e) = result {
            self.metrics.block_publish_errors.fetch_add(1, Ordering::Relaxed);
            error!("Failed to publish block: {}", e);
            return Err(e);
        }
        
        self.metrics.blocks_published.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
    
    fn publish_entries(&self, entries: &[EntryData]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }
        
        let batches = Self::batch_data(entries, self.batch_size);
        let batches_count = batches.len() as u64;
        
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        
        for batch in batches {
            let result = runtime.block_on(
                self.publish_message(ENTRY_TOPIC, batch)
            );
            
            if let Err(e) = result {
                self.metrics.entry_publish_errors.fetch_add(1, Ordering::Relaxed);
                error!("Failed to publish entry batch: {}", e);
                return Err(e);
            }
        }
        
        self.metrics.entry_batches_published.fetch_add(batches_count, Ordering::Relaxed);
        Ok(())
    }
}

impl NetworkPublisher {
    pub fn publish_blocks(&self, blocks: &[BlockData]) -> Result<()> {
        for block in blocks {
            self.publish_block(block.clone())?;
        }
        Ok(())
    }
}

async fn create_mock_node(config: NodeConfig) -> Result<(Node, tokio::sync::mpsc::Sender<()>)> {
    // Use the create_simple implementation we just added
    Node::create_simple(config).await
}

impl Publisher for Arc<NetworkPublisher> {
    fn publish_accounts(&self, accounts: &[AccountData]) -> Result<()> {
        (**self).publish_accounts(accounts)
    }
    
    fn publish_transactions(&self, transactions: &[TransactionData]) -> Result<()> {
        (**self).publish_transactions(transactions)
    }
    
    fn publish_block(&self, block: BlockData) -> Result<()> {
        (**self).publish_block(block)
    }
    
    fn publish_entries(&self, entries: &[EntryData]) -> Result<()> {
        (**self).publish_entries(entries)
    }
}
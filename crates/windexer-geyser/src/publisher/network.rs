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
    log::{error, warn},
    serde::{Deserialize, Serialize},
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

#[derive(Clone, Debug)]
pub struct NetworkPublisher {
    batch_size: usize,
    metrics: Arc<Metrics>,
    validator_id: Option<String>,
    shutdown: Arc<ShutdownFlag>,
}

impl NetworkPublisher {
    pub async fn new(config: PublisherConfig, shutdown: Arc<ShutdownFlag>) -> Result<Self> {
        let env_var = std::env::var("WINDEXER_SKIP_NETWORK").unwrap_or_default();
        warn!("WINDEXER_SKIP_NETWORK value: '{}', is set: {}", env_var, env_var == "1");
        warn!("Creating network publisher with network disabled");
        Ok(Self {
            batch_size: config.batch_size,
            metrics: config.metrics,
            validator_id: config.validator_id,
            shutdown,
        })
    }
    
    fn batch_data<T>(data: &[T], batch_size: usize) -> Vec<Vec<T>> 
    where
        T: Clone,
    {
        if batch_size == 0 {
            return vec![data.to_vec()];
        }
        
        let mut result = Vec::new();
        let mut current_batch = Vec::new();
        
        for item in data {
            current_batch.push(item.clone());
            
            if current_batch.len() >= batch_size {
                result.push(current_batch);
                current_batch = Vec::new();
            }
        }
        
        if !current_batch.is_empty() {
            result.push(current_batch);
        }
        
        result
    }
}

impl Publisher for NetworkPublisher {
    fn publish_accounts(&self, accounts: &[AccountData]) -> Result<()> {
        if accounts.is_empty() {
            return Ok(());
        }
        
        let batches = Self::batch_data(accounts, self.batch_size);
        let batches_count = batches.len() as u64;
        
        self.metrics.account_batches_published.fetch_add(batches_count, Ordering::Relaxed);
        Ok(())
    }
    
    fn publish_transactions(&self, transactions: &[TransactionData]) -> Result<()> {
        if transactions.is_empty() {
            return Ok(());
        }
        
        let batches = Self::batch_data(transactions, self.batch_size);
        let batches_count = batches.len() as u64;
        
        self.metrics.transaction_batches_published.fetch_add(batches_count, Ordering::Relaxed);
        Ok(())
    }
    
    fn publish_block(&self, _block: BlockData) -> Result<()> {
        self.metrics.blocks_published.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
    
    fn publish_entries(&self, entries: &[EntryData]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }
        
        let batches = Self::batch_data(entries, self.batch_size);
        let batches_count = batches.len() as u64;
        
        self.metrics.entry_batches_published.fetch_add(batches_count, Ordering::Relaxed);
        Ok(())
    }
}

use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use windexer_common::{
    helius::{HeliusClient, HeliusConfig},
    types::helius::{AccountData, BlockData, TransactionData},
};

/// Data fetcher that uses Helius to get blockchain data
#[derive(Debug)]
pub struct HeliusDataFetcher {
    client: Arc<HeliusClient>,
    cache: Arc<RwLock<DataCache>>,
}

/// Cache for blockchain data
#[derive(Default, Debug)]
struct DataCache {
    accounts: std::collections::HashMap<String, AccountData>,
    blocks: std::collections::HashMap<u64, BlockData>,
    transactions: std::collections::HashMap<String, TransactionData>,
    latest_slot: u64,
}

impl HeliusDataFetcher {
    /// Create a new data fetcher with the given API key
    pub fn new(api_key: &str) -> Self {
        let config = HeliusConfig {
            api_key: api_key.to_string(),
            network: "mainnet".to_string(),
            ws_endpoint: None,
            http_endpoint: None,
        };
        
        Self {
            client: Arc::new(HeliusClient::new(config)),
            cache: Arc::new(RwLock::new(DataCache::default())),
        }
    }
    
    /// Create a new data fetcher with the given configuration
    pub fn new_with_config(config: HeliusConfig) -> Self {
        Self {
            client: Arc::new(HeliusClient::new(config)),
            cache: Arc::new(RwLock::new(DataCache::default())),
        }
    }
    
    /// Initialize the data fetcher
    pub async fn initialize(&self) -> Result<()> {
        // Connect to WebSocket to receive updates
        self.client.connect_websocket().await?;
        
        // Subscribe to slot updates
        self.client.subscribe_slots().await?;
        
        // Set up a task to update the cache with latest block info
        let client = self.client.clone();
        let cache = self.cache.clone();
        
        tokio::spawn(async move {
            let mut block_rx = client.subscribe_block_updates();
            
            while let Ok(block) = block_rx.recv().await {
                let mut cache_guard = cache.write().await;
                cache_guard.blocks.insert(block.slot, block.clone());
                cache_guard.latest_slot = block.slot;
                
                // If the cache gets too large, remove old entries
                if cache_guard.blocks.len() > 1000 {
                    let oldest_slots: Vec<u64> = cache_guard.blocks.keys()
                        .copied()
                        .collect();
                    
                    // Sort and keep only the latest 500 slots
                    if oldest_slots.len() > 500 {
                        let mut oldest_slots = oldest_slots;
                        oldest_slots.sort_unstable();
                        let to_remove = &oldest_slots[..oldest_slots.len() - 500];
                        
                        for slot in to_remove {
                            cache_guard.blocks.remove(slot);
                        }
                    }
                }
            }
        });
        
        // Set up a task to update the accounts cache
        let client = self.client.clone();
        let cache = self.cache.clone();
        
        tokio::spawn(async move {
            let mut account_rx = client.subscribe_account_updates();
            
            while let Ok(account) = account_rx.recv().await {
                let mut cache_guard = cache.write().await;
                cache_guard.accounts.insert(account.pubkey.clone(), account);
                
                // If the cache gets too large, remove old entries
                if cache_guard.accounts.len() > 10000 {
                    // Remove random entries until we're back to a reasonable size
                    while cache_guard.accounts.len() > 5000 {
                        if let Some(key) = cache_guard.accounts.keys().next().cloned() {
                            cache_guard.accounts.remove(&key);
                        } else {
                            break;
                        }
                    }
                }
            }
        });
        
        // Set up a task to update the transactions cache
        let client = self.client.clone();
        let cache = self.cache.clone();
        
        tokio::spawn(async move {
            let mut tx_rx = client.subscribe_transaction_updates();
            
            while let Ok(tx) = tx_rx.recv().await {
                let mut cache_guard = cache.write().await;
                cache_guard.transactions.insert(tx.signature.clone(), tx);
                
                // If the cache gets too large, remove old entries
                if cache_guard.transactions.len() > 10000 {
                    // Remove random entries until we're back to a reasonable size
                    while cache_guard.transactions.len() > 5000 {
                        if let Some(key) = cache_guard.transactions.keys().next().cloned() {
                            cache_guard.transactions.remove(&key);
                        } else {
                            break;
                        }
                    }
                }
            }
        });
        
        // Fetch the latest slot to initialize the cache
        let latest_slot = self.client.get_latest_slot().await?;
        
        // Fetch the latest block
        let latest_block = self.client.get_block(latest_slot).await?;
        
        // Update the cache
        let mut cache_guard = self.cache.write().await;
        cache_guard.latest_slot = latest_slot;
        cache_guard.blocks.insert(latest_slot, latest_block);
        
        Ok(())
    }
    
    /// Get account data by public key
    pub async fn get_account(&self, pubkey: &str) -> Result<AccountData> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(account) = cache.accounts.get(pubkey) {
                return Ok(account.clone());
            }
        }
        
        // Fetch from Helius
        let account = self.client.get_account(pubkey).await?;
        
        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.accounts.insert(pubkey.to_string(), account.clone());
        }
        
        Ok(account)
    }
    
    /// Get block data by slot
    pub async fn get_block(&self, slot: u64) -> Result<BlockData> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(block) = cache.blocks.get(&slot) {
                return Ok(block.clone());
            }
        }
        
        // Fetch from Helius
        let block = self.client.get_block(slot).await?;
        
        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.blocks.insert(slot, block.clone());
        }
        
        Ok(block)
    }
    
    /// Get transaction data by signature
    pub async fn get_transaction(&self, signature: &str) -> Result<TransactionData> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(tx) = cache.transactions.get(signature) {
                return Ok(tx.clone());
            }
        }
        
        // Fetch from Helius
        let tx = self.client.get_transaction(signature).await?;
        
        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.transactions.insert(signature.to_string(), tx.clone());
        }
        
        Ok(tx)
    }
    
    /// Get the latest slot
    pub async fn get_latest_slot(&self) -> Result<u64> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if cache.latest_slot > 0 {
                return Ok(cache.latest_slot);
            }
        }
        
        // Fetch from Helius
        let slot = self.client.get_latest_slot().await?;
        
        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.latest_slot = slot;
        }
        
        Ok(slot)
    }
    
    /// Subscribe to updates for a specific account
    pub async fn subscribe_account(&self, pubkey: &str) -> Result<()> {
        self.client.subscribe_account(pubkey).await
    }
    
    /// Subscribe to updates for a specific program
    pub async fn subscribe_program(&self, program_id: &str) -> Result<()> {
        self.client.subscribe_program(program_id).await
    }
    
    /// Get the underlying Helius client
    pub fn client(&self) -> Arc<HeliusClient> {
        self.client.clone()
    }
} 
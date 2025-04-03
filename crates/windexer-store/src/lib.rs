//! This is the windexer-store crate - handles data storage and caching

mod internal;
pub mod traits;
pub mod factory;
pub mod parquet_store;
pub mod postgres_store;

// Re-export for backward compatibility
pub use internal::*;

use {
    traits::Storage,
    async_trait::async_trait,
    anyhow::{anyhow, Result},
    std::{
        path::PathBuf,
        sync::{Arc, Mutex},
    },
    windexer_common::types::{
        account::AccountData,
        block::BlockData,
        transaction::TransactionData,
    },
};

pub struct StoreConfig {
    pub path: PathBuf,
    pub max_open_files: i32,
    pub cache_capacity: usize,
}

pub struct Store {
    // In a real implementation, this would be a database connection or similar
    config: StoreConfig,
    // Placeholder for database - this would be a real DB in production
    accounts: Arc<Mutex<Vec<AccountData>>>,
    transactions: Arc<Mutex<Vec<TransactionData>>>,
    blocks: Arc<Mutex<Vec<BlockData>>>,
}

impl Store {
    pub fn open(config: StoreConfig) -> Result<Self> {
        std::fs::create_dir_all(&config.path)?;
        
        Ok(Self {
            config,
            accounts: Arc::new(Mutex::new(Vec::new())),
            transactions: Arc::new(Mutex::new(Vec::new())),
            blocks: Arc::new(Mutex::new(Vec::new())),
        })
    }
    
    pub fn store_account(&self, account: AccountData) -> Result<()> {
        let mut accounts = self.accounts.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        accounts.push(account);
        Ok(())
    }
    
    pub fn store_transaction(&self, transaction: TransactionData) -> Result<()> {
        let mut transactions = self.transactions.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        transactions.push(transaction);
        Ok(())
    }
    
    pub fn store_block(&self, block: BlockData) -> Result<()> {
        let mut blocks = self.blocks.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        blocks.push(block);
        Ok(())
    }
    
    pub fn account_count(&self) -> usize {
        self.accounts.lock().unwrap().len()
    }
    
    pub fn transaction_count(&self) -> usize {
        self.transactions.lock().unwrap().len()
    }
    
    pub fn block_count(&self) -> usize {
        self.blocks.lock().unwrap().len()
    }
    
    pub fn get_recent_accounts(&self, limit: usize) -> Vec<AccountData> {
        let accounts = self.accounts.lock().unwrap();
        let start = if accounts.len() > limit {
            accounts.len() - limit
        } else {
            0
        };
        accounts[start..].to_vec()
    }
    
    pub fn get_recent_transactions(&self, limit: usize) -> Vec<TransactionData> {
        let transactions = self.transactions.lock().unwrap();
        let start = if transactions.len() > limit {
            transactions.len() - limit
        } else {
            0
        };
        transactions[start..].to_vec()
    }
    
    // Add methods for retrieving data, etc.
}

#[async_trait]
impl Storage for Store {
    async fn store_account(&self, account: AccountData) -> Result<()> {
        // Call the sync version in a way that doesn't block
        tokio::task::spawn_blocking(move || {
            let store = self.clone();
            store.store_account(account)
        }).await?
    }
    
    async fn store_transaction(&self, transaction: TransactionData) -> Result<()> {
        // Call the sync version in a way that doesn't block
        tokio::task::spawn_blocking(move || {
            let store = self.clone();
            store.store_transaction(transaction)
        }).await?
    }
    
    async fn store_block(&self, block: BlockData) -> Result<()> {
        // Call the sync version in a way that doesn't block
        tokio::task::spawn_blocking(move || {
            let store = self.clone();
            store.store_block(block)
        }).await?
    }
    
    async fn get_account(&self, pubkey: &str) -> Result<Option<AccountData>> {
        let pubkey = pubkey.to_string(); // Clone for moving into task
        let store = self.clone();
        
        // Call the sync version in a way that doesn't block
        tokio::task::spawn_blocking(move || {
            store.get_account(&pubkey)
        }).await?
    }
    
    async fn get_transaction(&self, signature: &str) -> Result<Option<TransactionData>> {
        let signature = signature.to_string(); // Clone for moving into task
        let store = self.clone();
        
        // Call the sync version in a way that doesn't block
        tokio::task::spawn_blocking(move || {
            store.get_transaction(&signature)
        }).await?
    }
    
    async fn get_block(&self, slot: u64) -> Result<Option<BlockData>> {
        let store = self.clone();
        
        // Call the sync version in a way that doesn't block
        tokio::task::spawn_blocking(move || {
            store.get_block(slot)
        }).await?
    }
    
    async fn get_recent_accounts(&self, limit: usize) -> Result<Vec<AccountData>> {
        let store = self.clone();
        
        // Call the sync version in a way that doesn't block
        tokio::task::spawn_blocking(move || {
            Ok(store.get_recent_accounts(limit))
        }).await?
    }
    
    async fn get_recent_transactions(&self, limit: usize) -> Result<Vec<TransactionData>> {
        let store = self.clone();
        
        // Call the sync version in a way that doesn't block
        tokio::task::spawn_blocking(move || {
            Ok(store.get_recent_transactions(limit))
        }).await?
    }
    
    async fn get_recent_blocks(&self, limit: usize) -> Result<Vec<BlockData>> {
        // For now, return empty since the sync API doesn't have this
        Ok(Vec::new())
    }
    
    async fn get_accounts_by_slot_range(&self, start_slot: u64, end_slot: u64, limit: usize) -> Result<Vec<AccountData>> {
        let store = self.clone();
        
        // Call the sync version in a way that doesn't block
        tokio::task::spawn_blocking(move || {
            store.get_accounts_by_slot_range(start_slot, end_slot, limit)
        }).await?
    }
    
    async fn get_transactions_by_slot_range(&self, start_slot: u64, end_slot: u64, limit: usize) -> Result<Vec<TransactionData>> {
        let store = self.clone();
        
        // Call the sync version in a way that doesn't block
        tokio::task::spawn_blocking(move || {
            store.get_transactions_by_slot_range(start_slot, end_slot, limit)
        }).await?
    }
    
    async fn get_blocks_by_slot_range(&self, start_slot: u64, end_slot: u64, limit: usize) -> Result<Vec<BlockData>> {
        // For now, return empty since the sync API doesn't have this
        Ok(Vec::new())
    }
    
    async fn close(&self) -> Result<()> {
        // No explicit close needed for RocksDB
        Ok(())
    }
}

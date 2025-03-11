//! This is the windexer-store crate - handles data storage and caching

use {
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

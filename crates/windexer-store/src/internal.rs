use {
    anyhow::{anyhow, Result},
    std::{
        path::{Path, PathBuf},
        sync::Arc,
    },
    rocksdb::{
        DB, Options, ReadOptions, WriteBatch, ColumnFamilyDescriptor, Cache, 
        DBCompressionType, BlockBasedOptions, SliceTransform,
    },
    windexer_common::types::{
        AccountData,
        TransactionData,
        BlockData,
    },
};

pub const CF_ACCOUNTS: &str = "accounts";
pub const CF_TRANSACTIONS: &str = "transactions";
pub const CF_BLOCKS: &str = "blocks";
pub const CF_METADATA: &str = "metadata";

#[derive(Clone, Debug)]
pub struct StoreConfig {
    pub path: PathBuf,
    pub max_open_files: i32,
    pub cache_capacity: usize,
}

#[derive(Clone)]
pub struct Store {
    db: Arc<DB>,
}

impl Store {
    pub fn open(config: StoreConfig) -> Result<Self> {
        let path = config.path.clone();
        
        // Create directory if it doesn't exist
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }
        
        // Configure database options
        let mut options = Options::default();
        options.create_if_missing(true);
        options.create_missing_column_families(true);
        options.set_max_open_files(config.max_open_files);
        options.set_compression_type(DBCompressionType::Lz4);
        options.set_bottommost_compression_type(DBCompressionType::Zstd);
        options.increase_parallelism(num_cpus::get() as i32);
        
        // Configure block-based table options
        let mut block_opts = BlockBasedOptions::default();
        let cache = Cache::new_lru_cache(config.cache_capacity);
        block_opts.set_block_cache(&cache);
        block_opts.set_pin_l0_filter_and_index_blocks_in_cache(true);
        block_opts.set_cache_index_and_filter_blocks(true);
        options.set_block_based_table_factory(&block_opts);
        
        // Define column families
        let cf_opts = options.clone();
        let cf_accounts = ColumnFamilyDescriptor::new(CF_ACCOUNTS, cf_opts.clone());
        let cf_transactions = ColumnFamilyDescriptor::new(CF_TRANSACTIONS, cf_opts.clone());
        let cf_blocks = ColumnFamilyDescriptor::new(CF_BLOCKS, cf_opts.clone());
        let cf_metadata = ColumnFamilyDescriptor::new(CF_METADATA, cf_opts.clone());
        
        // Open database
        let db = DB::open_cf_descriptors(
            &options, 
            &path, 
            vec![cf_accounts, cf_transactions, cf_blocks, cf_metadata]
        )?;
        
        Ok(Self {
            db: Arc::new(db),
        })
    }
    
    pub fn store_account(&self, account: AccountData) -> Result<()> {
        let cf = self.db.cf_handle(CF_ACCOUNTS)
            .ok_or_else(|| anyhow!("Column family '{}' not found", CF_ACCOUNTS))?;
        
        // Serialize account to byte array
        let data = bincode::serialize(&account)?;
        
        // Store in RocksDB
        self.db.put_cf(&cf, account.pubkey.as_bytes(), &data)?;
        
        Ok(())
    }
    
    pub fn store_transaction(&self, transaction: TransactionData) -> Result<()> {
        let cf = self.db.cf_handle(CF_TRANSACTIONS)
            .ok_or_else(|| anyhow!("Column family '{}' not found", CF_TRANSACTIONS))?;
        
        // Serialize transaction to byte array
        let data = bincode::serialize(&transaction)?;
        
        // Store in RocksDB
        self.db.put_cf(&cf, transaction.signature.as_bytes(), &data)?;
        
        Ok(())
    }
    
    pub fn store_block(&self, block: BlockData) -> Result<()> {
        let cf = self.db.cf_handle(CF_BLOCKS)
            .ok_or_else(|| anyhow!("Column family '{}' not found", CF_BLOCKS))?;
        
        // Serialize block to byte array
        let data = bincode::serialize(&block)?;
        
        // Store in RocksDB using slot as key
        let key = block.slot.to_be_bytes();
        self.db.put_cf(&cf, &key, &data)?;
        
        Ok(())
    }
    
    pub fn get_account(&self, pubkey: &str) -> Result<Option<AccountData>> {
        let cf = self.db.cf_handle(CF_ACCOUNTS)
            .ok_or_else(|| anyhow!("Column family '{}' not found", CF_ACCOUNTS))?;
        
        match self.db.get_cf(&cf, pubkey.as_bytes())? {
            Some(data) => {
                let account: AccountData = bincode::deserialize(&data)?;
                Ok(Some(account))
            },
            None => Ok(None),
        }
    }
    
    pub fn get_transaction(&self, signature: &str) -> Result<Option<TransactionData>> {
        let cf = self.db.cf_handle(CF_TRANSACTIONS)
            .ok_or_else(|| anyhow!("Column family '{}' not found", CF_TRANSACTIONS))?;
        
        match self.db.get_cf(&cf, signature.as_bytes())? {
            Some(data) => {
                let tx: TransactionData = bincode::deserialize(&data)?;
                Ok(Some(tx))
            },
            None => Ok(None),
        }
    }
    
    pub fn get_block(&self, slot: u64) -> Result<Option<BlockData>> {
        let cf = self.db.cf_handle(CF_BLOCKS)
            .ok_or_else(|| anyhow!("Column family '{}' not found", CF_BLOCKS))?;
        
        let key = slot.to_be_bytes();
        match self.db.get_cf(&cf, &key)? {
            Some(data) => {
                let block: BlockData = bincode::deserialize(&data)?;
                Ok(Some(block))
            },
            None => Ok(None),
        }
    }
    
    pub fn get_recent_accounts(&self, limit: usize) -> Result<Vec<AccountData>> {
        let cf = self.db.cf_handle(CF_ACCOUNTS)
            .ok_or_else(|| anyhow!("Column family '{}' not found", CF_ACCOUNTS))?;
        
        let mut accounts = Vec::with_capacity(limit);
        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::End);
        
        for (_, value) in iter.take(limit) {
            let account: AccountData = bincode::deserialize(&value)?;
            accounts.push(account);
        }
        
        Ok(accounts)
    }
    
    pub fn get_recent_transactions(&self, limit: usize) -> Result<Vec<TransactionData>> {
        let cf = self.db.cf_handle(CF_TRANSACTIONS)
            .ok_or_else(|| anyhow!("Column family '{}' not found", CF_TRANSACTIONS))?;
        
        let mut transactions = Vec::with_capacity(limit);
        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::End);
        
        for (_, value) in iter.take(limit) {
            let tx: TransactionData = bincode::deserialize(&value)?;
            transactions.push(tx);
        }
        
        Ok(transactions)
    }
    
    pub fn get_accounts_by_slot_range(&self, start_slot: u64, end_slot: u64, limit: usize) -> Result<Vec<AccountData>> {
        let cf = self.db.cf_handle(CF_ACCOUNTS)
            .ok_or_else(|| anyhow!("Column family '{}' not found", CF_ACCOUNTS))?;
        
        let mut accounts = Vec::with_capacity(limit);
        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
        
        for (_, value) in iter {
            if accounts.len() >= limit {
                break;
            }
            
            let account: AccountData = bincode::deserialize(&value)?;
            
            if account.slot >= start_slot && account.slot <= end_slot {
                accounts.push(account);
            }
        }
        
        Ok(accounts)
    }
    
    pub fn get_transactions_by_slot_range(&self, start_slot: u64, end_slot: u64, limit: usize) -> Result<Vec<TransactionData>> {
        let cf = self.db.cf_handle(CF_TRANSACTIONS)
            .ok_or_else(|| anyhow!("Column family '{}' not found", CF_TRANSACTIONS))?;
        
        let mut transactions = Vec::with_capacity(limit);
        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
        
        for (_, value) in iter {
            if transactions.len() >= limit {
                break;
            }
            
            let tx: TransactionData = bincode::deserialize(&value)?;
            
            if tx.slot >= start_slot && tx.slot <= end_slot {
                transactions.push(tx);
            }
        }
        
        Ok(transactions)
    }
} 
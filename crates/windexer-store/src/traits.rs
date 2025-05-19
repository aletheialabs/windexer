use {
    anyhow::Result,
    std::sync::Arc,
    async_trait::async_trait,
    windexer_common::{
        types::{
            AccountData,
            TransactionData,
            BlockData,
        },
    },
};

/// A trait representing the core storage capabilities required by wIndexer.
/// This abstraction allows for pluggable storage backends.
#[async_trait]
pub trait Storage: Send + Sync + 'static {
    /// Store an account update
    async fn store_account(&self, account: AccountData) -> Result<()>;
    
    /// Store a transaction
    async fn store_transaction(&self, transaction: TransactionData) -> Result<()>;
    
    /// Store a block
    async fn store_block(&self, block: BlockData) -> Result<()>;
    
    /// Get account by public key
    async fn get_account(&self, pubkey: &str) -> Result<Option<AccountData>>;
    
    /// Get transaction by signature
    async fn get_transaction(&self, signature: &str) -> Result<Option<TransactionData>>;
    
    /// Get block by slot
    async fn get_block(&self, slot: u64) -> Result<Option<BlockData>>;
    
    /// Get recent accounts up to a limit
    async fn get_recent_accounts(&self, limit: usize) -> Result<Vec<AccountData>>;
    
    /// Get recent transactions up to a limit
    async fn get_recent_transactions(&self, limit: usize) -> Result<Vec<TransactionData>>;
    
    /// Get recent blocks up to a limit
    async fn get_recent_blocks(&self, limit: usize) -> Result<Vec<BlockData>>;
    
    /// Get accounts by slot range
    async fn get_accounts_by_slot_range(&self, start_slot: u64, end_slot: u64, limit: usize) -> Result<Vec<AccountData>>;
    
    /// Get transactions by slot range
    async fn get_transactions_by_slot_range(&self, start_slot: u64, end_slot: u64, limit: usize) -> Result<Vec<TransactionData>>;
    
    /// Get blocks by slot range
    async fn get_blocks_by_slot_range(&self, start_slot: u64, end_slot: u64, limit: usize) -> Result<Vec<BlockData>>;
    
    /// Close the storage (flush any pending writes, close connections, etc.)
    async fn close(&self) -> Result<()>;
}

/// Factory trait for creating storage instances
#[async_trait]
pub trait StorageFactory: Send + Sync + 'static {
    /// Create a new storage instance with the given configuration
    async fn create_storage(&self) -> Result<Arc<dyn Storage>>;
} 
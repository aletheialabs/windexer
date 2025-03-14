use {
    super::Publisher,
    anyhow::Result,
    windexer_common::types::{
        account::AccountData,
        transaction::TransactionData,
        block::BlockData,
        block::EntryData,
    },
};

pub struct NullPublisher;

impl NullPublisher {
    pub fn new() -> Self {
        Self
    }
}

impl Publisher for NullPublisher {
    fn publish_accounts(&self, _accounts: &[AccountData]) -> Result<()> {
        Ok(())
    }
    
    fn publish_transactions(&self, _transactions: &[TransactionData]) -> Result<()> {
        Ok(())
    }
    
    fn publish_block(&self, _block: BlockData) -> Result<()> {
        Ok(())
    }
    
    fn publish_entries(&self, _entries: &[EntryData]) -> Result<()> {
        Ok(())
    }
} 
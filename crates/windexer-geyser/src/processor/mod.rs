// crates/windexer-geyser/src/processor/mod.rs

//! Data processing module
//!
//! This module contains the core data processing logic for handling accounts, transactions,
//! and blocks from the Geyser plugin interface.

mod account;
mod transaction;
mod block;

pub use account::AccountProcessor;
pub use transaction::TransactionProcessor;
pub use block::BlockProcessor;

use {
    crate::{
        config::{AccountsSelector, TransactionSelector},
        metrics::Metrics,
        ShutdownFlag,
    },
    agave_geyser_plugin_interface::geyser_plugin_interface::{
        ReplicaAccountInfoVersions, ReplicaBlockInfoVersions,
        ReplicaTransactionInfoVersions, ReplicaEntryInfoVersions, SlotStatus,
    },
    solana_sdk::{clock::Slot, pubkey::Pubkey},
    anyhow::Result,
    crossbeam_channel::{Sender, Receiver, bounded, unbounded},
    std::{
        sync::{Arc, atomic::{AtomicBool, Ordering}},
        thread::{self, JoinHandle},
    },
    crate::publisher::Publisher,
};

#[derive(Clone)]
pub struct ProcessorConfig {
    pub thread_count: usize,
    
    pub batch_size: usize,
    
    pub metrics: Arc<Metrics>,
    
    pub shutdown_flag: Arc<ShutdownFlag>,
}

pub trait AccountHandler: Send + 'static {
    fn process_account(
        &self,
        account: ReplicaAccountInfoVersions,
        slot: Slot,
        is_startup: bool,
    ) -> Result<()>;
    
    fn notify_end_of_startup(&self) -> Result<()>;
}

pub trait TransactionHandler: Send + 'static {
    fn process_transaction(
        &self,
        transaction: ReplicaTransactionInfoVersions,
        slot: Slot,
    ) -> Result<()>;
}

pub trait BlockHandler: Send + Sync {
    fn update_slot_status(
        &self,
        slot: Slot,
        parent: Option<Slot>,
        status: SlotStatus,
    ) -> Result<()>;
    
    fn process_block_metadata(
        &self,
        block_info: ReplicaBlockInfoVersions,
    ) -> Result<()>;
    
    fn process_entry(
        &self,
        entry_info: ReplicaEntryInfoVersions,
    ) -> Result<()>;
}

pub struct ProcessorHandle<T> {
    processor: Arc<T>,
    workers: Vec<JoinHandle<()>>,
}

impl<T> ProcessorHandle<T> {
    pub fn new(processor: T, workers: Vec<JoinHandle<()>>) -> Self {
        Self {
            processor: Arc::new(processor),
            workers,
        }
    }
    
    pub fn join(self) {
        for worker in self.workers {
            let _ = worker.join();
        }
    }
}

impl<T: AccountHandler> ProcessorHandle<T> {
    pub fn process_account(
        &self,
        account: ReplicaAccountInfoVersions,
        slot: Slot,
        is_startup: bool,
    ) -> Result<()> {
        self.processor.process_account(account, slot, is_startup)
    }
    
    pub fn notify_end_of_startup(&self) -> Result<()> {
        self.processor.notify_end_of_startup()
    }
}

impl<T: TransactionHandler> ProcessorHandle<T> {
    pub fn process_transaction(
        &self,
        transaction: ReplicaTransactionInfoVersions,
        slot: Slot,
    ) -> Result<()> {
        self.processor.process_transaction(transaction, slot)
    }
}

impl<T: BlockHandler> ProcessorHandle<T> {
    pub fn update_slot_status(
        &self,
        slot: Slot,
        parent: Option<Slot>,
        status: SlotStatus,
    ) -> Result<()> {
        self.processor.update_slot_status(slot, parent, status)
    }
    
    pub fn process_block_metadata(
        &self,
        block_info: ReplicaBlockInfoVersions,
    ) -> Result<()> {
        self.processor.process_block_metadata(block_info)
    }
    
    pub fn process_entry(
        &self,
        entry_info: ReplicaEntryInfoVersions,
    ) -> Result<()> {
        self.processor.process_entry(entry_info)
    }
}
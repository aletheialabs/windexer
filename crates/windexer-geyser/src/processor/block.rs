//! Block data processor
//!
//! This module contains the implementation of the block data processor.

use {
    crate::{
        metrics::Metrics,
        processor::{ProcessorConfig, BlockHandler, ProcessorHandle},
        publisher::Publisher,
        ShutdownFlag,
    },
    agave_geyser_plugin_interface::geyser_plugin_interface::{
        ReplicaBlockInfoVersions, ReplicaBlockInfo, ReplicaBlockInfoV2, ReplicaBlockInfoV3,
        ReplicaEntryInfoVersions, ReplicaEntryInfo, ReplicaEntryInfoV2,
        SlotStatus,
    },
    solana_sdk::{
        clock::Slot,
        reward_type::RewardType,
        pubkey::Pubkey,
    },
    solana_transaction_status::Reward,
    anyhow::{anyhow, Result},
    crossbeam_channel::{Sender, Receiver, bounded},
    log::{debug, error, info, trace, warn},
    std::{
        collections::HashMap,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            Mutex, RwLock,
        },
        thread::{self, JoinHandle},
        time::Duration,
    },
    windexer_common::types::{
        block::BlockData,
        block::EntryData,
    },
};

enum BlockMessage {
    UpdateSlotStatus {
        slot: Slot,
        parent: Option<Slot>,
        status: SlotStatus,
    },
    
    ProcessBlockMetadata {
        block_info_slot: Slot,
        blockhash: String,
        rewards: Vec<Reward>,
        block_time: Option<i64>,
        block_height: Option<u64>,
        parent_slot: Option<Slot>,
        transaction_count: Option<u64>,
        entry_count: Option<u64>,
    },
    
    ProcessEntry {
        slot: Slot,
        index: u64,
        num_hashes: u64,
        hash: Vec<u8>,
        executed_transaction_count: u64,
        starting_transaction_index: Option<u64>,
    },
    
    Shutdown,
}

pub struct BlockProcessor {
    config: ProcessorConfig,
    publisher: Arc<dyn Publisher>,
    sender: Sender<BlockMessage>,    
    receivers: Vec<Receiver<BlockMessage>>,
    tracked_slots: Arc<RwLock<HashMap<Slot, BlockData>>>,
}

impl BlockProcessor {
    pub fn new(
        config: ProcessorConfig,
        publisher: Arc<dyn Publisher>,
    ) -> ProcessorHandle<Self> {
        let (sender, receivers) = Self::create_channels(config.thread_count);
        
        let processor = Self {
            config: config.clone(),
            publisher,
            sender,
            receivers,
            tracked_slots: Arc::new(RwLock::new(HashMap::new())),
        };
        
        let workers = processor.start_workers();
        
        ProcessorHandle::new(processor, workers)
    }
    
    fn create_channels(
        thread_count: usize,
    ) -> (Sender<BlockMessage>, Vec<Receiver<BlockMessage>>) {
        let (sender, main_receiver) = bounded(10_000);
        let mut receivers = Vec::with_capacity(thread_count);
        
        for _ in 0..thread_count {
            let (worker_sender, worker_receiver) = bounded(1_000);
            
            let main_receiver_clone = main_receiver.clone();
            thread::spawn(move || {
                for message in main_receiver_clone.iter() {
                    match &message {
                        BlockMessage::Shutdown => {
                            let _ = worker_sender.send(message);
                            break;
                        }
                        _ => {
                            if worker_sender.try_send(message).is_err() {
                                // If the channel is full, just drop the message
                                // The worker is probably busy and we don't want to block
                                // the main thread
                            }
                        }
                    }
                }
            });
            
            receivers.push(worker_receiver);
        }
        
        (sender, receivers)
    }
    
    fn start_workers(&self) -> Vec<JoinHandle<()>> {
        let mut workers = Vec::with_capacity(self.receivers.len());
        
        for (i, receiver) in self.receivers.iter().enumerate() {
            let receiver = receiver.clone();
            let publisher = self.publisher.clone();
            let metrics = self.config.metrics.clone();
            let shutdown_flag = self.config.shutdown_flag.clone();
            let tracked_slots = self.tracked_slots.clone();
            
            let worker = thread::Builder::new()
                .name(format!("block-worker-{}", i))
                .spawn(move || {
                    Self::worker_thread(
                        receiver,
                        publisher,
                        metrics,
                        shutdown_flag,
                        tracked_slots,
                    );
                })
                .unwrap();
            
            workers.push(worker);
        }
        
        workers
    }
    
    fn worker_thread(
        receiver: Receiver<BlockMessage>,
        publisher: Arc<dyn Publisher>,
        metrics: Arc<Metrics>,
        shutdown_flag: Arc<ShutdownFlag>,
        tracked_slots: Arc<RwLock<HashMap<Slot, BlockData>>>,
    ) {
        let mut entry_batch = Vec::new();
        let mut last_publish = std::time::Instant::now();
        
        let mut last_cleanup = std::time::Instant::now();
        
        for message in receiver.iter() {
            if shutdown_flag.is_shutdown() {
                break;
            }
            
            // Process message
            match message {
                BlockMessage::UpdateSlotStatus { slot, parent, status } => {
                    // Get or create block data for this slot
                    let mut slots = tracked_slots.write().unwrap();
                    let block_data = slots.entry(slot).or_insert_with(|| BlockData {
                        slot,
                        parent_slot: parent,
                        status: status.clone(),
                        blockhash: None,
                        rewards: Some(vec![]),
                        timestamp: None,
                        block_height: None,
                        transaction_count: None,
                        entry_count: 0,
                        entries: vec![],
                        parent_blockhash: None,
                    });
                    
                    block_data.status = status.clone();
                    
                    if matches!(status, SlotStatus::Rooted) {
                        if let Err(e) = publisher.publish_block(block_data.clone()) {
                            error!("Failed to publish block: {}", e);
                            metrics.block_publish_errors.fetch_add(1, Ordering::Relaxed);
                        } else {
                            metrics.blocks_published.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                BlockMessage::ProcessBlockMetadata { block_info_slot, blockhash, rewards, block_time, block_height, parent_slot, transaction_count: _transaction_count, entry_count } => {
                    // Convert block info
                    let block_data = BlockData {
                        slot: block_info_slot,
                        parent_slot: parent_slot,
                        status: SlotStatus::Processed,
                        blockhash: Some(blockhash),
                        rewards: Some(rewards.iter().map(|_r| {
                            Reward {
                                pubkey: "Unknown".to_string(),
                                lamports: 0,
                                post_balance: 0,
                                reward_type: None,
                                commission: None,
                            }
                        }).collect()),
                        timestamp: block_time,
                        block_height,
                        transaction_count: Some(0),
                        entry_count: entry_count.unwrap_or(0),
                        entries: vec![],
                        parent_blockhash: None,
                    };
                    
                    let mut slots = tracked_slots.write().unwrap();
                    let existing = slots.entry(block_info_slot).or_insert_with(|| BlockData {
                        slot: block_info_slot,
                        parent_slot: None, // Will be updated from block info
                        status: SlotStatus::Processed,
                        blockhash: None,
                        rewards: Some(vec![]),
                        timestamp: None,
                        block_height: None,
                        transaction_count: None,
                        entry_count: 0,
                        entries: vec![],
                        parent_blockhash: None,
                    });
                    
                    existing.blockhash = block_data.blockhash;
                    existing.rewards = block_data.rewards;
                    existing.timestamp = block_data.timestamp;
                    existing.block_height = block_data.block_height;
                    existing.transaction_count = block_data.transaction_count;
                    existing.entry_count = block_data.entry_count;
                    
                    if block_data.parent_slot.is_some() {
                        existing.parent_slot = block_data.parent_slot;
                    }
                    
                    if matches!(existing.status, SlotStatus::Rooted) {
                        if let Err(e) = publisher.publish_block(existing.clone()) {
                            error!("Failed to publish block: {}", e);
                            metrics.block_publish_errors.fetch_add(1, Ordering::Relaxed);
                        } else {
                            metrics.blocks_published.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                BlockMessage::ProcessEntry { slot, index, num_hashes, hash, executed_transaction_count, starting_transaction_index } => {
                    let entry_data = EntryData {
                        slot,
                        index: index as usize,
                        num_hashes,
                        hash,
                        executed_transaction_count,
                        starting_transaction_index: starting_transaction_index.map(|idx| idx as usize).unwrap_or(0),
                    };
                    
                    entry_batch.push(entry_data.clone());
                    
                    let mut slots = tracked_slots.write().unwrap();
                    let block_data = slots.entry(slot).or_insert_with(|| BlockData {
                        slot,
                        parent_slot: None,
                        status: SlotStatus::Processed,
                        blockhash: None,
                        rewards: Some(vec![]),
                        timestamp: None,
                        block_height: None,
                        transaction_count: None,
                        entry_count: 0,
                        entries: vec![],
                        parent_blockhash: None,
                    });
                    
                    block_data.entry_count += 1;
                    block_data.entries.push(entry_data);
                    
                    if entry_batch.len() >= 1000 || last_publish.elapsed() > Duration::from_millis(100) {
                        if !entry_batch.is_empty() {
                            if let Err(e) = publisher.publish_entries(&entry_batch) {
                                error!("Failed to publish entries: {}", e);
                                metrics.entry_publish_errors.fetch_add(1, Ordering::Relaxed);
                            } else {
                                metrics.entry_batches_published.fetch_add(entry_batch.len() as u64, Ordering::Relaxed);
                            }
                            entry_batch.clear();
                            last_publish = std::time::Instant::now();
                        }
                    }
                }
                BlockMessage::Shutdown => {
                    debug!("Block worker received shutdown message");
                    break;
                }
            }
            
            if last_cleanup.elapsed() > Duration::from_secs(60) {
                Self::cleanup_old_slots(&tracked_slots);
                last_cleanup = std::time::Instant::now();
            }
        }
        
        if !entry_batch.is_empty() {
            if let Err(e) = publisher.publish_entries(&entry_batch) {
                error!("Failed to publish entries: {}", e);
                metrics.entry_publish_errors.fetch_add(1, Ordering::Relaxed);
            } else {
                metrics.entry_batches_published.fetch_add(entry_batch.len() as u64, Ordering::Relaxed);
            }
        }
        
        debug!("Block worker thread exiting");
    }
    
    fn cleanup_old_slots(tracked_slots: &Arc<RwLock<HashMap<Slot, BlockData>>>) {
        let mut slots_to_remove = Vec::new();
        let _now = std::time::Instant::now();
        
        {
            let slots = tracked_slots.read().unwrap();
            for (slot, block_data) in slots.iter() {
                if matches!(block_data.status, SlotStatus::Rooted) && *slot < slots.len().saturating_sub(1000) as u64 {
                    slots_to_remove.push(*slot);
                }
            }
        }
        
        if !slots_to_remove.is_empty() {
            let mut slots = tracked_slots.write().unwrap();
            for slot in slots_to_remove {
                slots.remove(&slot);
            }
        }
    }
}

impl BlockHandler for BlockProcessor {
    fn update_slot_status(
        &self,
        slot: Slot,
        parent: Option<Slot>,
        status: SlotStatus,
    ) -> Result<()> {
        self.sender.send(BlockMessage::UpdateSlotStatus {
            slot,
            parent,
            status,
        })?;
        
        Ok(())
    }
    
    fn process_block_metadata(
        &self,
        block_info: ReplicaBlockInfoVersions,
    ) -> Result<()> {
        let (slot, blockhash, rewards, block_time, block_height, parent_slot, _transaction_count, entry_count) = 
            match &block_info {
                ReplicaBlockInfoVersions::V0_0_1(info) => {
                    (info.slot, 
                     info.blockhash.to_string(),
                     info.rewards.to_vec(),
                     info.block_time,
                     info.block_height,
                     None,
                     None,
                     None)
                },
                ReplicaBlockInfoVersions::V0_0_2(info) => {
                    (info.slot, 
                     info.blockhash.to_string(),
                     info.rewards.to_vec(),
                     info.block_time,
                     info.block_height,
                     Some(info.parent_slot),
                     Some(info.executed_transaction_count),
                     None)
                },
                ReplicaBlockInfoVersions::V0_0_3(info) => {
                    (info.slot, 
                     info.blockhash.to_string(),
                     info.rewards.to_vec(),
                     info.block_time,
                     info.block_height,
                     Some(info.parent_slot),
                     Some(info.executed_transaction_count),
                     Some(info.entry_count))
                },
                ReplicaBlockInfoVersions::V0_0_4(info) => {
                    (info.slot, 
                     info.blockhash.to_string(),
                     info.rewards.rewards.clone(),
                     info.block_time,
                     info.block_height,
                     Some(info.parent_slot),
                     Some(info.executed_transaction_count),
                     None)
                },
            };
        
        let converted_rewards: Vec<Reward> = rewards.into_iter().map(|_r| Reward {
            pubkey: "Unknown".to_string(),
            lamports: 0,
            post_balance: 0,
            reward_type: None,
            commission: None,
        }).collect();

        self.sender.send(BlockMessage::ProcessBlockMetadata {
            block_info_slot: slot,
            blockhash,
            rewards: converted_rewards,
            block_time,
            block_height,
            parent_slot,
            transaction_count: _transaction_count,
            entry_count,
        })?;
        
        Ok(())
    }
    
    fn process_entry(
        &self,
        entry_info: ReplicaEntryInfoVersions,
    ) -> Result<()> {
        let (slot, index, num_hashes, hash, tx_count, starting_index) = 
            match &entry_info {
                ReplicaEntryInfoVersions::V0_0_1(info) => {
                    (info.slot, 
                     info.index,
                     info.num_hashes,
                     info.hash.to_vec(),
                     info.executed_transaction_count,
                     None)
                },
                ReplicaEntryInfoVersions::V0_0_2(info) => {
                    (info.slot, 
                     info.index,
                     info.num_hashes,
                     info.hash.to_vec(),
                     info.executed_transaction_count,
                     Some(info.starting_transaction_index as u64))
                },
            };
            
        self.sender.send(BlockMessage::ProcessEntry {
            slot,
            index: index as u64,
            num_hashes,
            hash,
            executed_transaction_count: tx_count,
            starting_transaction_index: starting_index,
        })?;
        
        Ok(())
    }
}
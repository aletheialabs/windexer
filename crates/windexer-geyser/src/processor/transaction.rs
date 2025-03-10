//! Transaction data processor
//!
//! This module contains the implementation of the transaction data processor.

use {
    crate::{
        config::TransactionSelector,
        metrics::Metrics,
        processor::{ProcessorConfig, TransactionHandler, ProcessorHandle},
        publisher::Publisher,
        ShutdownFlag,
    },
    agave_geyser_plugin_interface::geyser_plugin_interface::{
        ReplicaTransactionInfoVersions,
    },
    solana_transaction_status::{
        TransactionStatusMeta,
    },
    solana_sdk::{
        clock::Slot,
        pubkey::Pubkey,
        transaction::SanitizedTransaction,
        signature::Signature,
        hash::Hash as Blockhash,
        transaction::TransactionError,
        instruction::InstructionError,
        program_utils::limited_deserialize,
        address_lookup_table::state::AddressLookupTable,
        transaction::VersionedTransaction,
        message::v0::LoadedAddresses,
        message::Message,
    },
    anyhow::{anyhow, Result},
    crossbeam_channel::{Sender, Receiver, bounded},
    log::{debug, error, info, trace, warn},
    std::{
        collections::HashSet,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            RwLock,
        },
        thread::{self, JoinHandle},
        time::Duration,
        str::FromStr,
    },
    windexer_common::types::transaction::TransactionData,
};

enum TransactionMessage {
    ProcessTransaction {
        signature: [u8; 64],
        slot: Slot,
        is_vote: bool,
    },
    
    Shutdown,
}

pub struct TransactionProcessor {
    config: ProcessorConfig,
    publisher: Arc<dyn Publisher>,
    selector: Option<TransactionSelector>,
    mentioned_accounts: Arc<RwLock<Option<HashSet<Pubkey>>>>,
    include_all_transactions: Arc<AtomicBool>,
    include_votes: Arc<AtomicBool>,
    sender: Sender<TransactionMessage>,
    receivers: Vec<Receiver<TransactionMessage>>,
}

impl TransactionProcessor {
    pub fn new(
        config: ProcessorConfig,
        publisher: Arc<dyn Publisher>,
        selector: Option<TransactionSelector>,
    ) -> ProcessorHandle<Self> {
        let (mentioned_accounts, include_all_transactions, include_votes) = 
            Self::parse_selectors(&selector);
        
        let (sender, receivers) = Self::create_channels(config.thread_count);
        
        let processor = Self {
            config: config.clone(),
            publisher,
            selector,
            mentioned_accounts: Arc::new(RwLock::new(mentioned_accounts)),
            include_all_transactions: Arc::new(AtomicBool::new(include_all_transactions)),
            include_votes: Arc::new(AtomicBool::new(include_votes)),
            sender,
            receivers,
        };
        
        let workers = processor.start_workers();
        
        ProcessorHandle::new(processor, workers)
    }
    
    fn parse_selectors(
        selector: &Option<TransactionSelector>,
    ) -> (Option<HashSet<Pubkey>>, bool, bool) {
        let mut mentioned_accounts = None;
        let mut include_all_transactions = false;
        let mut include_votes = false;
        
        if let Some(selector) = selector {
            if selector.mentions.contains(&"*".to_string()) {
                include_all_transactions = true;
            } else if selector.mentions.contains(&"all_votes".to_string()) {
                include_votes = true;
            } else {
                let mut account_set = HashSet::new();
                for mention in &selector.mentions {
                    if let Ok(pubkey) = Pubkey::from_str(mention) {
                        account_set.insert(pubkey);
                    } else {
                        warn!("Invalid mention pubkey in selector: {}", mention);
                    }
                }
                mentioned_accounts = Some(account_set);
            }
            
            if selector.include_votes {
                include_votes = true;
            }
        }
        
        (mentioned_accounts, include_all_transactions, include_votes)
    }
    
    fn create_channels(
        thread_count: usize,
    ) -> (Sender<TransactionMessage>, Vec<Receiver<TransactionMessage>>) {
        let (sender, main_receiver) = bounded(10_000);
        let mut receivers = Vec::with_capacity(thread_count);
        
        for _ in 0..thread_count {
            let (worker_sender, worker_receiver) = bounded(1_000);
            
            let main_receiver_clone = main_receiver.clone();
            thread::spawn(move || {
                for message in main_receiver_clone.iter() {
                    match &message {
                        TransactionMessage::Shutdown => {
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
    
    /// Start worker threads
    fn start_workers(&self) -> Vec<JoinHandle<()>> {
        let mut workers = Vec::with_capacity(self.receivers.len());
        
        for (i, receiver) in self.receivers.iter().enumerate() {
            let receiver = receiver.clone();
            let publisher = self.publisher.clone();
            let metrics = self.config.metrics.clone();
            let shutdown_flag = self.config.shutdown_flag.clone();
            let mentioned_accounts = self.mentioned_accounts.clone();
            let include_all_transactions = self.include_all_transactions.clone();
            let include_votes = self.include_votes.clone();
            
            let worker = thread::Builder::new()
                .name(format!("transaction-worker-{}", i))
                .spawn(move || {
                    Self::worker_thread(
                        receiver,
                        publisher,
                        metrics,
                        shutdown_flag,
                        mentioned_accounts,
                        include_all_transactions,
                        include_votes,
                    );
                })
                .unwrap();
            
            workers.push(worker);
        }
        
        workers
    }
    
    fn worker_thread(
        receiver: Receiver<TransactionMessage>,
        publisher: Arc<dyn Publisher>,
        metrics: Arc<Metrics>,
        shutdown_flag: Arc<ShutdownFlag>,
        mentioned_accounts: Arc<RwLock<Option<HashSet<Pubkey>>>>,
        include_all_transactions: Arc<AtomicBool>,
        include_votes: Arc<AtomicBool>,
    ) {
        let mut batch = Vec::new();
        let mut last_publish = std::time::Instant::now();
        
        for message in receiver.iter() {
            if shutdown_flag.is_shutdown() {
                break;
            }
            
            match message {
                TransactionMessage::ProcessTransaction { signature, slot, is_vote } => {
                    if !Self::should_process_transaction(
                        &signature, 
                        &is_vote,
                        &mentioned_accounts, 
                        &include_all_transactions,
                        &include_votes,
                    ) {
                        continue;
                    }
                    
                    match Self::convert_transaction(signature, slot, is_vote) {
                        Ok(transaction_data) => {
                            batch.push(transaction_data);
                            
                            if batch.len() >= 1000 || last_publish.elapsed() > Duration::from_millis(100) {
                                if !batch.is_empty() {
                                    if let Err(e) = publisher.publish_transactions(&batch) {
                                        error!("Failed to publish transactions: {}", e);
                                        metrics.transaction_publish_errors.fetch_add(1, Ordering::Relaxed);
                                    } else {
                                        metrics.transaction_batches_published.fetch_add(batch.len() as u64, Ordering::Relaxed);
                                    }
                                    batch.clear();
                                    last_publish = std::time::Instant::now();
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to convert transaction: {}", e);
                            metrics.transaction_update_errors.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                TransactionMessage::Shutdown => {
                    debug!("Transaction worker received shutdown message");
                    break;
                }
            }
        }
        
        if !batch.is_empty() {
            if let Err(e) = publisher.publish_transactions(&batch) {
                error!("Failed to publish transactions: {}", e);
                metrics.transaction_publish_errors.fetch_add(1, Ordering::Relaxed);
            } else {
                metrics.transaction_batches_published.fetch_add(batch.len() as u64, Ordering::Relaxed);
            }
        }
        
        debug!("Transaction worker thread exiting");
    }
    
    fn should_process_transaction(
        signature: &[u8; 64],
        is_vote: &bool,
        mentioned_accounts: &Arc<RwLock<Option<HashSet<Pubkey>>>>,
        include_all_transactions: &Arc<AtomicBool>,
        include_votes: &Arc<AtomicBool>,
    ) -> bool {
        if include_all_transactions.load(Ordering::Relaxed) {
            return true;
        }
        
        if *is_vote && include_votes.load(Ordering::Relaxed) {
            return true;
        }
        
        if let Some(_accounts) = mentioned_accounts.read().unwrap().as_ref() {
            for _account_key in signature.iter() {
                // ...
            }
        }
        
        false
    }

    fn convert_transaction(
        signature: [u8; 64],
        slot: Slot,
        is_vote: bool,
    ) -> Result<TransactionData> {
        Ok(TransactionData {
            signature: Signature::default(),
            slot,
            is_vote,
            message: Message::new_with_blockhash(
                &[],
                None,
                &Blockhash::default(),
            ),
            signatures: vec![Signature::from(signature)],
            meta: TransactionStatusMeta {
                status: Ok(()),
                fee: 0,
                pre_balances: vec![],
                post_balances: vec![],
                inner_instructions: None,
                log_messages: None,
                pre_token_balances: None,
                post_token_balances: None,
                rewards: None,
                loaded_addresses: LoadedAddresses::default(),
                return_data: None,
                compute_units_consumed: None,
            },
            serializable_meta: (&TransactionStatusMeta {
                status: Ok(()),
                fee: 0,
                pre_balances: vec![],
                post_balances: vec![],
                inner_instructions: None,
                log_messages: None,
                pre_token_balances: None,
                post_token_balances: None,
                rewards: None,
                loaded_addresses: LoadedAddresses::default(),
                return_data: None,
                compute_units_consumed: None,
            }).into(),
            index: 0, // Unknown in V1
        })
    }
}

impl TransactionHandler for TransactionProcessor {
    fn process_transaction(
        &self,
        transaction: ReplicaTransactionInfoVersions,
        slot: Slot,
    ) -> Result<()> {
        let signature_bytes = match &transaction {
            ReplicaTransactionInfoVersions::V0_0_1(info) => {
                let mut bytes = [0u8; 64];
                bytes.copy_from_slice(info.signature.as_ref());
                bytes
            },
            ReplicaTransactionInfoVersions::V0_0_2(info) => {
                let mut bytes = [0u8; 64];
                bytes.copy_from_slice(info.signature.as_ref());
                bytes
            },
        };
        
        let is_vote = match &transaction {
            ReplicaTransactionInfoVersions::V0_0_1(info) => info.is_vote,
            ReplicaTransactionInfoVersions::V0_0_2(info) => info.is_vote,
        };
        
        self.sender.send(TransactionMessage::ProcessTransaction {
            signature: signature_bytes,
            slot,
            is_vote,
        }).map_err(|e| anyhow!("Failed to send transaction to processor: {}", e))
    }
}
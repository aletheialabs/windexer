// crates/windexer-geyser/src/processor/account.rs

//! Account data processor
//!
//! This module contains the implementation of the account data processor.

use {
    crate::{
        config::AccountsSelector,
        metrics::Metrics,
        processor::{ProcessorConfig, AccountHandler, ProcessorHandle},
        publisher::Publisher,
        ShutdownFlag,
    },
    agave_geyser_plugin_interface::geyser_plugin_interface::ReplicaAccountInfoVersions,
    solana_sdk::{
        clock::Slot,
        pubkey::Pubkey,
    },
    agave_geyser_plugin_interface::geyser_plugin_interface::{
        GeyserPluginError, ReplicaAccountInfo, ReplicaAccountInfoV2, ReplicaAccountInfoV3
    },
    anyhow::{anyhow, Result},
    crossbeam_channel::{Sender, Receiver, bounded},
    log::{debug, error, info, trace, warn},
    std::{
        collections::HashSet,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            Mutex, RwLock,
        },
        thread::{self, JoinHandle},
        time::Duration,
        str::FromStr,
    },
    windexer_common::types::account::AccountData,
};

enum AccountMessage {
    ProcessAccount {
        pubkey: Pubkey,
        lamports: u64,
        owner: Pubkey,
        executable: bool,
        rent_epoch: u64,
        data: Vec<u8>,
        write_version: u64,
        slot: Slot,
        is_startup: bool,
    },
    
    EndOfStartup,
    
    Shutdown,
}

pub struct AccountProcessor {
    config: ProcessorConfig,
    publisher: Arc<dyn Publisher>,
    selector: Option<AccountsSelector>,
    included_accounts: Arc<RwLock<Option<HashSet<Pubkey>>>>,
    included_owners: Arc<RwLock<Option<HashSet<Pubkey>>>>,
    include_all_accounts: Arc<AtomicBool>,
    sender: Sender<AccountMessage>,
    receivers: Vec<Receiver<AccountMessage>>,
    startup_complete: Arc<AtomicBool>,
}

impl AccountProcessor {
    pub fn new(
        config: ProcessorConfig,
        publisher: Arc<dyn Publisher>,
        selector: Option<AccountsSelector>,
    ) -> ProcessorHandle<Self> {
        let (included_accounts, included_owners, include_all_accounts) = 
            Self::parse_selectors(&selector);
        
        let (sender, receivers) = Self::create_channels(config.thread_count);
        
        let processor = Self {
            config: config.clone(),
            publisher,
            selector,
            included_accounts: Arc::new(RwLock::new(included_accounts)),
            included_owners: Arc::new(RwLock::new(included_owners)),
            include_all_accounts: Arc::new(AtomicBool::new(include_all_accounts)),
            sender,
            receivers,
            startup_complete: Arc::new(AtomicBool::new(false)),
        };
        
        let workers = processor.start_workers();
        
        ProcessorHandle::new(processor, workers)
    }
    
    fn parse_selectors(
        selector: &Option<AccountsSelector>,
    ) -> (Option<HashSet<Pubkey>>, Option<HashSet<Pubkey>>, bool) {
        let mut included_accounts = None;
        let mut included_owners = None;
        let mut include_all_accounts = false;
        
        if let Some(selector) = selector {
            if let Some(accounts) = &selector.accounts {
                if accounts.contains(&"*".to_string()) {
                    include_all_accounts = true;
                } else {
                    let mut account_set = HashSet::new();
                    for account in accounts {
                        if let Ok(pubkey) = Pubkey::from_str(account) {
                            account_set.insert(pubkey);
                        } else {
                            warn!("Invalid account pubkey in selector: {}", account);
                        }
                    }
                    included_accounts = Some(account_set);
                }
            }
            
            if let Some(owners) = &selector.owners {
                let mut owner_set = HashSet::new();
                for owner in owners {
                    if let Ok(pubkey) = Pubkey::from_str(owner) {
                        owner_set.insert(pubkey);
                    } else {
                        warn!("Invalid owner pubkey in selector: {}", owner);
                    }
                }
                if !owner_set.is_empty() {
                    included_owners = Some(owner_set);
                }
            }
        } else {
            included_accounts = Some(HashSet::new());
        }
        
        (included_accounts, included_owners, include_all_accounts)
    }
    
    /// Create channels for workers
    fn create_channels(
        thread_count: usize,
    ) -> (Sender<AccountMessage>, Vec<Receiver<AccountMessage>>) {
        let (sender, main_receiver) = bounded(10_000);
        let mut receivers = Vec::with_capacity(thread_count);
        
        for _ in 0..thread_count {
            let (worker_sender, worker_receiver) = bounded(1_000);
            
            let main_receiver_clone = main_receiver.clone();
            thread::spawn(move || {
                for message in main_receiver_clone.iter() {
                    match &message {
                        AccountMessage::Shutdown => {
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
            let included_accounts = self.included_accounts.clone();
            let included_owners = self.included_owners.clone();
            let include_all_accounts = self.include_all_accounts.clone();
            let startup_complete = self.startup_complete.clone();
            
            let worker = thread::Builder::new()
                .name(format!("account-worker-{}", i))
                .spawn(move || {
                    Self::worker_thread(
                        receiver,
                        publisher,
                        metrics,
                        shutdown_flag,
                        included_accounts,
                        included_owners,
                        include_all_accounts,
                        startup_complete,
                    );
                })
                .unwrap();
            
            workers.push(worker);
        }
        
        workers
    }
    
    fn worker_thread(
        receiver: Receiver<AccountMessage>,
        publisher: Arc<dyn Publisher>,
        metrics: Arc<Metrics>,
        shutdown_flag: Arc<ShutdownFlag>,
        included_accounts: Arc<RwLock<Option<HashSet<Pubkey>>>>,
        included_owners: Arc<RwLock<Option<HashSet<Pubkey>>>>,
        include_all_accounts: Arc<AtomicBool>,
        startup_complete: Arc<AtomicBool>,
    ) {
        let mut batch = Vec::new();
        let mut last_publish = std::time::Instant::now();
        
        for message in receiver.iter() {
            if shutdown_flag.is_shutdown() {
                break;
            }
            
            match message {
                AccountMessage::ProcessAccount { pubkey, lamports, owner, executable, rent_epoch, data, write_version, slot, is_startup } => {
                    if !Self::should_process_account(
                        &pubkey, 
                        &included_accounts, 
                        &included_owners,
                        &include_all_accounts,
                    ) {
                        continue;
                    }
                    
                    match Self::convert_account(pubkey, lamports, owner, executable, rent_epoch, data, write_version, slot, is_startup) {
                        Ok(account_data) => {
                            batch.push(account_data);
                            
                            if batch.len() >= 1000 || last_publish.elapsed() > Duration::from_millis(100) {
                                if !batch.is_empty() {
                                    if let Err(e) = publisher.publish_accounts(&batch) {
                                        error!("Failed to publish accounts: {}", e);
                                        metrics.account_publish_errors.fetch_add(1, Ordering::Relaxed);
                                    } else {
                                        metrics.account_batches_published.fetch_add(batch.len() as u64, Ordering::Relaxed);
                                    }
                                    batch.clear();
                                    last_publish = std::time::Instant::now();
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to convert account: {}", e);
                            metrics.account_update_errors.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                AccountMessage::EndOfStartup => {
                    info!("End of startup notification received by account worker");
                    startup_complete.store(true, Ordering::SeqCst);
                    
                    if !batch.is_empty() {
                        if let Err(e) = publisher.publish_accounts(&batch) {
                            error!("Failed to publish accounts: {}", e);
                            metrics.account_publish_errors.fetch_add(1, Ordering::Relaxed);
                        } else {
                            metrics.account_batches_published.fetch_add(batch.len() as u64, Ordering::Relaxed);
                        }
                        batch.clear();
                    }
                }
                AccountMessage::Shutdown => {
                    debug!("Account worker received shutdown message");
                    break;
                }
            }
        }
        
        if !batch.is_empty() {
            if let Err(e) = publisher.publish_accounts(&batch) {
                error!("Failed to publish accounts: {}", e);
                metrics.account_publish_errors.fetch_add(1, Ordering::Relaxed);
            } else {
                metrics.account_batches_published.fetch_add(batch.len() as u64, Ordering::Relaxed);
            }
        }
        
        debug!("Account worker thread exiting");
    }
    
    fn should_process_account(
        pubkey: &Pubkey,
        included_accounts: &Arc<RwLock<Option<HashSet<Pubkey>>>>,
        included_owners: &Arc<RwLock<Option<HashSet<Pubkey>>>>,
        include_all_accounts: &Arc<AtomicBool>,
    ) -> bool {
        if include_all_accounts.load(Ordering::Relaxed) {
            return true;
        }
        
        if let Some(included) = included_accounts.read().unwrap().as_ref() {
            if included.contains(pubkey) {
                return true;
            }
        }
        
        if let Some(included) = included_owners.read().unwrap().as_ref() {
            if included.contains(pubkey) {
                return true;
            }
        }
        
        false
    }
    
    fn convert_account(
        pubkey: Pubkey,
        lamports: u64,
        owner: Pubkey,
        executable: bool,
        rent_epoch: u64,
        data: Vec<u8>,
        write_version: u64,
        slot: Slot,
        is_startup: bool,
    ) -> Result<AccountData> {
        Ok(AccountData {
            pubkey,
            lamports,
            owner,
            executable,
            rent_epoch,
            data,
            write_version,
            slot,
            is_startup,
            transaction_signature: None,
        })
    }
}

impl AccountHandler for AccountProcessor {
    fn process_account(
        &self,
        account: ReplicaAccountInfoVersions,
        slot: Slot,
        is_startup: bool,
    ) -> Result<()> {
        // Extract data from the account reference
        let (pubkey, lamports, owner, executable, rent_epoch, data, write_version) = 
            match &account {
                ReplicaAccountInfoVersions::V0_0_1(info) => {
                    // Create proper arrays for Pubkey construction
                    let mut pubkey_array = [0u8; 32];
                    pubkey_array.copy_from_slice(info.pubkey);
                    let pubkey = Pubkey::new_from_array(pubkey_array);
                    
                    let mut owner_array = [0u8; 32];
                    owner_array.copy_from_slice(info.owner);
                    let owner = Pubkey::new_from_array(owner_array);
                    
                    let data = info.data.to_vec();
                    (pubkey, info.lamports, owner, info.executable, info.rent_epoch, data, info.write_version)
                },
                ReplicaAccountInfoVersions::V0_0_2(info) => {
                    let mut pubkey_array = [0u8; 32];
                    pubkey_array.copy_from_slice(info.pubkey);
                    let pubkey = Pubkey::new_from_array(pubkey_array);
                    
                    let mut owner_array = [0u8; 32];
                    owner_array.copy_from_slice(info.owner);
                    let owner = Pubkey::new_from_array(owner_array);
                    
                    let data = info.data.to_vec();
                    (pubkey, info.lamports, owner, info.executable, info.rent_epoch, data, info.write_version)
                },
                ReplicaAccountInfoVersions::V0_0_3(info) => {
                    let mut pubkey_array = [0u8; 32];
                    pubkey_array.copy_from_slice(info.pubkey);
                    let pubkey = Pubkey::new_from_array(pubkey_array);
                    
                    let mut owner_array = [0u8; 32];
                    owner_array.copy_from_slice(info.owner);
                    let owner = Pubkey::new_from_array(owner_array);
                    
                    let data = info.data.to_vec();
                    (pubkey, info.lamports, owner, info.executable, info.rent_epoch, data, info.write_version)
                },
            };
        
        self.sender.send(AccountMessage::ProcessAccount {
            pubkey,
            lamports,
            owner,
            executable,
            rent_epoch,
            data,
            write_version,
            slot,
            is_startup,
        })?;
        
        Ok(())
    }
    
    fn notify_end_of_startup(&self) -> Result<()> {
        self.sender.send(AccountMessage::EndOfStartup)
            .map_err(|e| anyhow!("Failed to send end of startup notification: {}", e))
    }
}
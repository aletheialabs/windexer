//! wIndexer Geyser Plugin implementation
//!
//! This module contains the implementation of the GeyserPlugin trait for the wIndexer plugin.

use {
    crate::{
        config::GeyserPluginConfig,
        processor::{
            AccountProcessor, BlockProcessor, TransactionProcessor,
            ProcessorHandle, ProcessorConfig,
        },
        publisher::{Publisher, NetworkPublisher, PublisherConfig, NullPublisher},
        metrics::Metrics,
        ShutdownFlag, PluginVersion,
    },
    agave_geyser_plugin_interface::{
        geyser_plugin_interface::{
            GeyserPlugin, ReplicaAccountInfoVersions, ReplicaBlockInfoVersions,
            ReplicaTransactionInfoVersions, ReplicaEntryInfoVersions, SlotStatus,
            GeyserPluginError,
        },
    },
    log::{Log, LevelFilter, error, info, warn},
    solana_sdk::clock::Slot,
    std::{
        fmt::{Debug, Formatter, Result as FmtResult},
        sync::{Arc, Mutex, RwLock},
        str::FromStr,
    },
    tokio::runtime::Runtime,
    anyhow::{anyhow, Result},
    windexer_network::Node as NetworkNode,
    windexer_common::config::NodeConfig,
    windexer_common::SerializableKeypair,
};

#[derive(Debug)]
struct PluginState {
    config: GeyserPluginConfig,
    publisher: Arc<dyn Publisher>,
    runtime: Option<Runtime>,
}

pub struct WindexerGeyserPlugin {
    config: GeyserPluginConfig,
    metrics: Arc<Metrics>,
    account_processor: Arc<Mutex<Option<ProcessorHandle<AccountProcessor>>>>,
    transaction_processor: Arc<Mutex<Option<ProcessorHandle<TransactionProcessor>>>>,
    block_processor: Arc<Mutex<Option<ProcessorHandle<BlockProcessor>>>>,
    publisher: Arc<Mutex<Arc<dyn Publisher>>>,
    shutdown_flag: Arc<ShutdownFlag>,
    runtime: Arc<Mutex<Option<Runtime>>>,
    network_node: Arc<Mutex<Option<NetworkNode>>>,
    version: PluginVersion,
    initialized: Arc<std::sync::atomic::AtomicBool>,
    plugin_state: Arc<RwLock<Option<PluginState>>>,
}

impl WindexerGeyserPlugin {
    pub fn new() -> Self {
        let metrics = Arc::new(Metrics::new());
        let shutdown_flag = Arc::new(ShutdownFlag::new());
        
        Self {
            config: GeyserPluginConfig::default(),
            metrics: metrics.clone(),
            account_processor: Arc::new(Mutex::new(None)),
            transaction_processor: Arc::new(Mutex::new(None)),
            block_processor: Arc::new(Mutex::new(None)),
            publisher: Arc::new(Mutex::new(Arc::new(NullPublisher::new()))),
            shutdown_flag,
            runtime: Arc::new(Mutex::new(None)),
            network_node: Arc::new(Mutex::new(None)),
            version: PluginVersion::new(),
            initialized: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            plugin_state: Arc::new(RwLock::new(None)),
        }
    }

    fn initialize(&mut self, config_path: &str) -> Result<(), GeyserPluginError> {
        let config = match GeyserPluginConfig::load_from_file(config_path) {
            Ok(config) => config,
            Err(e) => {
                return Err(GeyserPluginError::ConfigFileReadError {
                    msg: format!("Failed to load config: {}", e),
                });
            }
        };
        
        config.validate()
            .map_err(|e| GeyserPluginError::ConfigFileReadError {
                msg: format!("Invalid config: {}", e),
            })?;
        
        let runtime = Runtime::new()
            .map_err(|e| GeyserPluginError::Custom(
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Error message: {}", e)))
            ))?;
        
        let _node_pubkey = if let Some(pubkey_str) = config.node_pubkey.clone() {
            let pubkey = solana_sdk::pubkey::Pubkey::from_str(&pubkey_str)
                .map_err(|e| {
                    GeyserPluginError::Custom(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Invalid node pubkey: {}", e)
                    )))
                })?;
            Some(pubkey)
        } else {
            None
        };
        
        let (network_node, _shutdown_sender) = runtime.block_on(async {
            let node_config = NodeConfig {
                node_id: config.network.node_id.clone(),
                listen_addr: config.network.listen_addr,
                rpc_addr: config.network.rpc_addr,
                bootstrap_peers: config.network.bootstrap_peers.clone(),
                data_dir: config.network.data_dir.clone(),
                keypair: SerializableKeypair::default(),
                metrics_addr: config.network.metrics_addr,
                geyser_plugin_config: config.network.geyser_plugin_config.clone(),
                solana_rpc_url: config.network.solana_rpc_url.clone(),
            };
            
            NetworkNode::create_simple(node_config)
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to create network node: {}", e);
                    GeyserPluginError::Custom(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other, 
                        error_msg
                    )))
                })
        })?;
        
        let publisher_config = PublisherConfig::new(
            config.network.listen_addr.to_string(),
            config.network.bootstrap_peers.clone(),
            Some(config.network.solana_rpc_url.clone()),
            config.batch_size,
            self.metrics.clone(),
            Some(config.network.node_id.clone()),
        );

        let publisher = runtime.block_on(async {
            NetworkPublisher::new(publisher_config, self.shutdown_flag.clone())
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to create network publisher: {}", e);
                    GeyserPluginError::Custom(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other, 
                        error_msg
                    )))
                })
        })?;
        
        let processor_config = ProcessorConfig {
            thread_count: config.thread_count,
            batch_size: config.batch_size,
            metrics: self.metrics.clone(),
            shutdown_flag: self.shutdown_flag.clone(),
        };
        
        let account_processor = AccountProcessor::new(
            processor_config.clone(),
            Arc::new(publisher.clone()),
            config.accounts_selector.clone(),
        );
        
        let transaction_processor = TransactionProcessor::new(
            processor_config.clone(),
            Arc::new(publisher.clone()),
            config.transaction_selector.clone(),
        );
        
        let block_processor = BlockProcessor::new(
            processor_config.clone(),
            Arc::new(publisher.clone()),
        );
        
        // Store all components
        *self.runtime.lock().unwrap() = Some(runtime);
        *self.network_node.lock().unwrap() = Some(network_node);
        *self.publisher.lock().unwrap() = Arc::new(publisher);
        *self.account_processor.lock().unwrap() = Some(account_processor);
        *self.transaction_processor.lock().unwrap() = Some(transaction_processor);
        *self.block_processor.lock().unwrap() = Some(block_processor);
        self.config = config;
        
        let runtime_handle = self.runtime.lock().unwrap();
        let runtime = runtime_handle.as_ref().unwrap();
        
        if let Some(node) = self.network_node.lock().unwrap().as_mut() {
            runtime.block_on(async {
                node.start().await.map_err(|e| GeyserPluginError::Custom(
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to start network node: {}", e)))
                ))?;
                Ok::<(), GeyserPluginError>(())
            })?;
        }
        
        self.initialized.store(true, std::sync::atomic::Ordering::SeqCst);
        
        info!("wIndexer Geyser plugin initialized successfully");
        info!("Plugin version: {:?}", self.version);
        
        Ok(())
    }

    fn cleanup(&mut self) {
        self.shutdown_flag.shutdown();

        if let Some(runtime) = self.runtime.lock().unwrap().as_ref() {
            runtime.block_on(async {
                if let Some(node) = self.network_node.lock().unwrap().as_ref() {
                    if let Err(e) = node.stop().await {
                        error!("Error stopping network node: {}", e);
                    }
                }
            });
        }
        
        if let Some(processor) = self.account_processor.lock().unwrap().take() {
            processor.join();
        }
        
        if let Some(processor) = self.transaction_processor.lock().unwrap().take() {
            processor.join();
        }
        
        if let Some(processor) = self.block_processor.lock().unwrap().take() {
            processor.join();
        }
        
        {
            let mut publisher_guard = self.publisher.lock().unwrap();
            *publisher_guard = Arc::new(NullPublisher::new());
        }
        
        {
            let mut node_guard = self.network_node.lock().unwrap();
            *node_guard = None;
        }
        
        {
            let mut runtime_guard = self.runtime.lock().unwrap();
            if let Some(runtime) = runtime_guard.take() {
                runtime.shutdown_timeout(std::time::Duration::from_secs(5));
            }
        }
        
        info!("wIndexer Geyser plugin cleanup completed");
    }

    fn debug_plugin_init(&self, stage: &str, message: &str) {
        info!("PLUGIN_INIT: {} - {}", stage, message);
    }
    
    pub fn load_plugin(&self, config_path: &str) -> Result<()> {
        info!("Loading wIndexer Geyser plugin with config path: {}", config_path);
        self.debug_plugin_init("ON_LOAD", "Started plugin loading");
        
        let mut config = match GeyserPluginConfig::load_from_file(config_path) {
            Ok(config) => {
                self.debug_plugin_init("CONFIG", "Successfully loaded config");
                config
            },
            Err(e) => {
                error!("Failed to load config: {}", e);
                return Err(anyhow::anyhow!("Failed to load config: {}", e));
            }
        };
        
        self.debug_plugin_init("PUBLISHER", "Creating publisher");
        
        let publisher = Arc::new(NullPublisher::new());
        
        self.debug_plugin_init("STATE", "Setting up plugin state");
        
        let plugin_state = PluginState {
            config,
            publisher,
            runtime: None,
        };
        
        *self.plugin_state.write().unwrap() = Some(plugin_state);
        
        self.debug_plugin_init("COMPLETE", "Plugin loaded successfully");
        Ok(())
    }
}

impl Debug for WindexerGeyserPlugin {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("WindexerGeyserPlugin")
            .field("version", &self.version)
            .field("initialized", &self.initialized.load(std::sync::atomic::Ordering::SeqCst))
            .finish()
    }
}

impl GeyserPlugin for WindexerGeyserPlugin {
    fn name(&self) -> &'static str {
        "windexer-geyser-plugin"
    }

    fn setup_logger(&self, logger: &'static dyn Log, level: LevelFilter) -> std::result::Result<(), GeyserPluginError> {
        log::set_max_level(level);
        if let Err(e) = log::set_logger(logger) {
            return Err(GeyserPluginError::Custom(Box::new(e)));
        }
        Ok(())
    }

    fn on_load(&mut self, config_file: &str, is_reload: bool) -> std::result::Result<(), GeyserPluginError> {
        info!("Loading wIndexer Geyser plugin with config path: {}", config_file);
        
        if is_reload {
            info!("Reloading wIndexer Geyser plugin");
            self.on_unload();
        }
        
        if let Err(e) = self.load_plugin(config_file) {
            return Err(GeyserPluginError::Custom(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            ))));
        }
        
        self.initialized.store(true, std::sync::atomic::Ordering::SeqCst);
        
        Ok(())
    }

    fn on_unload(&mut self) {
        info!("Unloading wIndexer Geyser plugin");
        
        self.initialized.store(false, std::sync::atomic::Ordering::SeqCst);
        
        self.cleanup();
    }

    fn update_account(&self, account: ReplicaAccountInfoVersions, slot: Slot, is_startup: bool) -> std::result::Result<(), GeyserPluginError> {
        if !self.initialized.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        
        self.metrics.account_updates.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        if let Some(processor) = self.account_processor.lock().unwrap().as_ref() {
            if let Err(err) = processor.process_account(account, slot, is_startup) {
                self.metrics.account_update_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let err_msg = format!("Failed to process account update: {}", err);
                
                if self.config.panic_on_error {
                    return Err(GeyserPluginError::AccountsUpdateError { msg: err_msg });
                } else {
                    error!("{}", err_msg);
                }
            }
        }
        
        Ok(())
    }

    fn notify_end_of_startup(&self) -> std::result::Result<(), GeyserPluginError> {
        if !self.initialized.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        
        info!("End of startup notification received");
        
        if let Some(processor) = self.account_processor.lock().unwrap().as_ref() {
            if let Err(err) = processor.notify_end_of_startup() {
                let err_msg = format!("Failed to process end of startup notification: {}", err);
                
                if self.config.panic_on_error {
                    return Err(GeyserPluginError::AccountsUpdateError { msg: err_msg });
                } else {
                    error!("{}", err_msg);
                }
            }
        }
        
        Ok(())
    }

    fn update_slot_status(&self, slot: Slot, parent: Option<Slot>, status: &SlotStatus) -> std::result::Result<(), GeyserPluginError> {
        if !self.initialized.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        
        self.metrics.block_updates.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics.block_update_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        if let Some(processor) = self.block_processor.lock().unwrap().as_ref() {
            if let Err(err) = processor.update_slot_status(slot, parent, status.clone()) {
                self.metrics.block_update_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let err_msg = format!("Failed to process slot status update: {}", err);
                
                if self.config.panic_on_error {
                    return Err(GeyserPluginError::SlotStatusUpdateError { msg: err_msg });
                } else {
                    error!("{}", err_msg);
                }
            }
        }
        
        Ok(())
    }

    fn notify_transaction(&self, transaction: ReplicaTransactionInfoVersions, slot: Slot) -> std::result::Result<(), GeyserPluginError> {
        if !self.initialized.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        
        self.metrics.transaction_updates.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        if let Some(processor) = self.transaction_processor.lock().unwrap().as_ref() {
            if let Err(err) = processor.process_transaction(transaction, slot) {
                self.metrics.transaction_update_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let _err_msg = format!("Failed to process transaction: {}", err);
                
                let boxed_error = Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{}", err)));
                return Err(GeyserPluginError::Custom(boxed_error));
            }
        }
        
        Ok(())
    }

    fn notify_block_metadata(&self, block_info: ReplicaBlockInfoVersions) -> std::result::Result<(), GeyserPluginError> {
        if !self.initialized.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        
        self.metrics.block_updates.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics.block_update_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        if let Some(processor) = self.block_processor.lock().unwrap().as_ref() {
            if let Err(err) = processor.process_block_metadata(block_info) {
                self.metrics.block_update_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let err_msg = format!("Failed to process block metadata: {}", err);
                
                if self.config.panic_on_error {
                    return Err(GeyserPluginError::Custom(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Error message: {}", err)))));
                } else {
                    error!("{}", err_msg);
                }
            }
        }
        
        Ok(())
    }

    fn notify_entry(&self, entry_info: ReplicaEntryInfoVersions) -> std::result::Result<(), GeyserPluginError> {
        if !self.initialized.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        
        self.metrics.entry_updates.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics.entry_updates_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        if let Some(processor) = self.block_processor.lock().unwrap().as_ref() {
            if let Err(err) = processor.process_entry(entry_info) {
                self.metrics.entry_updates_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let err_msg = format!("Failed to process entry: {}", err);
                
                if self.config.panic_on_error {
                    return Err(GeyserPluginError::Custom(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Error message: {}", err)))));
                } else {
                    error!("{}", err_msg);
                }
            }
        }
        
        Ok(())
    }

    fn account_data_notifications_enabled(&self) -> bool {
        if !self.initialized.load(std::sync::atomic::Ordering::SeqCst) {
            return false;
        }
        
        if let Some(_selector) = &self.config.accounts_selector {
            true
        } else {
            false
        }
    }

    fn transaction_notifications_enabled(&self) -> bool {
        if !self.initialized.load(std::sync::atomic::Ordering::SeqCst) {
            return false;
        }
        
        if let Some(_selector) = &self.config.transaction_selector {
            true
        } else {
            false
        }
    }

    fn entry_notifications_enabled(&self) -> bool {
        // Always enable entry notifications
        true
    }
}
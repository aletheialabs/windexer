use {
    anyhow::Result,
    clap::Parser,
    solana_sdk::{
        signer::keypair::Keypair,
        pubkey::Pubkey,
        signature::Signature,
        clock::Slot,
        message::Message,
        hash::Hash as Blockhash,
    },
    std::{
        collections::HashMap,
        path::PathBuf,
        sync::{Arc, Mutex},
        time::{Duration, Instant, SystemTime},
    },
    tokio::{
        sync::mpsc,
        time::interval,
    },
    tracing::{error, info, warn, debug},
    tracing_subscriber::{EnvFilter, fmt::format::FmtSpan},
    windexer_common::{
        config::NodeConfig,
        crypto::SerializableKeypair,
        types::{
            account::AccountData,
            block::BlockData,
            transaction::TransactionData,
        },
        utils::slot_status::SlotStatus,
    },
    windexer_network::{
        Node,
        gossip::{GossipMessage, MessageType},
    },
    windexer_store::{
        Store,
        StoreConfig,
    },
    solana_transaction_status::TransactionStatusMeta,
};

#[derive(Parser, Debug)]
#[clap(
    version, 
    about = "Local data generator for wIndexer",
    long_about = "Generates test data using local validator and Geyser plugin"
)]
struct Args {
    #[clap(short, long, default_value = "9000")]
    base_port: u16,
    
    #[clap(long, value_delimiter = ',')]
    bootstrap_peers: Vec<String>,

    #[clap(long, default_value = "http://localhost:8899")]
    solana_rpc: String,
    
    #[clap(long, default_value = "./data")]
    data_dir: String,
    
    #[clap(long, default_value = "30")]
    metrics_interval_seconds: u64,
    
    #[clap(long, default_value = "100")]
    num_accounts: usize,
    
    #[clap(long, default_value = "1000")]
    num_transactions: usize,
    
    #[clap(long, default_value = "100")]
    num_blocks: usize,

    #[clap(long, default_value = "1000")]
    max_slots: u64,
}

// Struct to track generation metrics
struct GenerationMetrics {
    start_time: Instant,
    accounts_generated: usize,
    transactions_generated: usize,
    blocks_generated: usize,
    last_slot: u64,
    accounts_per_second: f64,
    transactions_per_second: f64,
    blocks_per_second: f64,
}

impl Default for GenerationMetrics {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            accounts_generated: 0,
            transactions_generated: 0,
            blocks_generated: 0,
            last_slot: 0,
            accounts_per_second: 0.0,
            transactions_per_second: 0.0,
            blocks_per_second: 0.0,
        }
    }
}

impl GenerationMetrics {
    fn update_rates(&mut self) {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.accounts_per_second = self.accounts_generated as f64 / elapsed;
            self.transactions_per_second = self.transactions_generated as f64 / elapsed;
            self.blocks_per_second = self.blocks_generated as f64 / elapsed;
        }
    }
    
    fn log_metrics(&self) {
        info!(
            "📊 Generation metrics: {} accounts ({:.2}/s), {} txs ({:.2}/s), {} blocks ({:.2}/s), last slot: {}",
            self.accounts_generated,
            self.accounts_per_second,
            self.transactions_generated,
            self.transactions_per_second,
            self.blocks_generated,
            self.blocks_per_second,
            self.last_slot,
        );
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new("local_gen=info")
        });

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();

    let port = args.base_port;
    let rpc_port = port + 1000;
    let metrics_port = 9100;
    
    info!("🔧 Configuring local generator with ports:");
    info!("   P2P: {}", port);
    info!("   RPC: {}", rpc_port);
    info!("   Metrics: {}", metrics_port);

    let node_config = NodeConfig {
        node_id: "local_gen".to_string(),
        listen_addr: format!("127.0.0.1:{}", port).parse()?,
        rpc_addr: format!("127.0.0.1:{}", rpc_port).parse()?,
        bootstrap_peers: args.bootstrap_peers,
        data_dir: format!("{}/local_gen", args.data_dir),
        solana_rpc_url: args.solana_rpc,
        keypair: SerializableKeypair::new(&Keypair::new()),
        geyser_plugin_config: None,
        metrics_addr: Some(format!("127.0.0.1:{}", metrics_port).parse()?),
    };

    info!("🚀 Starting local data generator");
    let (mut node, shutdown_tx) = Node::create_simple(node_config).await?;
    
    let store_path = PathBuf::from(format!("{}/local_gen/store", args.data_dir));
    std::fs::create_dir_all(&store_path)?;
    
    let store_config = StoreConfig {
        path: store_path,
        max_open_files: 1000,
        cache_capacity: 100 * 1024 * 1024, // 100 MB
    };
    
    info!("💾 Initializing storage");
    let store = Store::open(store_config)?;
    let store = Arc::new(store);
    
    let (account_tx, mut account_rx) = mpsc::channel::<AccountData>(1000);
    let (tx_tx, tx_rx) = mpsc::channel::<TransactionData>(1000);
    let (block_tx, block_rx) = mpsc::channel::<BlockData>(1000);
    
    let metrics = Arc::new(Mutex::new(GenerationMetrics::default()));
    
    let metrics_clone = metrics.clone();
    let metrics_interval = args.metrics_interval_seconds;
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(metrics_interval));
        loop {
            interval.tick().await;
            
            let mut m = metrics_clone.lock().unwrap();
            m.update_rates();
            m.log_metrics();
        }
    });
    
    // Generate accounts
    let metrics_clone = metrics.clone();
    let num_accounts = args.num_accounts;
    let max_slots = args.max_slots;
    tokio::spawn(async move {
        for i in 0..num_accounts {
            let slot = (i as u64) % max_slots;
            let account = AccountData {
                pubkey: Pubkey::new_unique(),
                lamports: 1000,
                owner: Pubkey::new_unique(),
                executable: false,
                rent_epoch: 0,
                data: vec![1, 2, 3],
                write_version: 0,
                slot,
                is_startup: false,
                transaction_signature: None,
            };
            
            if let Err(e) = account_tx.send(account).await {
                error!("Failed to send account: {}", e);
            } else {
                let mut m = metrics_clone.lock().unwrap();
                m.accounts_generated += 1;
                m.last_slot = slot;
            }
        }
    });
    
    // Generate transactions
    let metrics_clone = metrics.clone();
    let num_transactions = args.num_transactions;
    let max_slots = args.max_slots;
    tokio::spawn(async move {
        for i in 0..num_transactions {
            let slot = (i as u64) % max_slots;
            let tx = TransactionData {
                signature: Signature::default(),
                slot,
                is_vote: false,
                message: Message::new_with_blockhash(
                    &[],
                    None,
                    &Blockhash::default(),
                ),
                signatures: vec![Signature::new_unique()],
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
                    loaded_addresses: solana_sdk::message::v0::LoadedAddresses::default(),
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
                    loaded_addresses: solana_sdk::message::v0::LoadedAddresses::default(),
                    return_data: None,
                    compute_units_consumed: None,
                }).into(),
                index: i,
            };
            
            if let Err(e) = tx_tx.send(tx).await {
                error!("Failed to send transaction: {}", e);
            } else {
                let mut m = metrics_clone.lock().unwrap();
                m.transactions_generated += 1;
                m.last_slot = slot;
            }
        }
    });
    
    // Generate blocks
    let metrics_clone = metrics.clone();
    let num_blocks = args.num_blocks;
    let max_slots = args.max_slots;
    tokio::spawn(async move {
        for i in 0..num_blocks {
            let slot = (i as u64) % max_slots;
            let timestamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            
            let block = BlockData {
                slot,
                parent_blockhash: if slot > 0 { Some(format!("dummy-parent-hash-{}", slot - 1)) } else { None },
                timestamp: Some(timestamp),
                transaction_count: Some(10),
                entry_count: 5,
                entries: vec![],
                blockhash: Some(format!("dummy-hash-{}", slot)),
                rewards: Some(Vec::new()),
                block_height: Some(slot),
                parent_slot: if slot > 0 { Some(slot - 1) } else { None },
                status: SlotStatus::Processed,
            };
            
            if let Err(e) = block_tx.send(block).await {
                error!("Failed to send block: {}", e);
            } else {
                let mut m = metrics_clone.lock().unwrap();
                m.blocks_generated += 1;
                m.last_slot = slot;
            }
        }
    });
    
    // Process accounts
    let store_clone = store.clone();
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        let mut account_rx = account_rx;
        while let Some(account) = account_rx.recv().await {
            debug!("Processing account: {}", account.pubkey);
            
            if let Err(e) = store_clone.store_account(account.clone()) {
                error!("Failed to store account: {}", e);
            }
        }
    });
    
    // Process transactions
    let store_clone = store.clone();
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        let mut tx_rx = tx_rx;
        while let Some(tx) = tx_rx.recv().await {
            debug!("Processing transaction: {}", tx.signature);
            
            if let Err(e) = store_clone.store_transaction(tx.clone()) {
                error!("Failed to store transaction: {}", e);
            } else {
                let mut m = metrics_clone.lock().unwrap();
                m.transactions_generated += 1;
                m.last_slot = tx.slot;
            }
        }
    });
    
    // Process blocks
    let store_clone = store.clone();
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        let mut block_rx = block_rx;
        while let Some(block) = block_rx.recv().await {
            info!("Processing block: {} (slot {})", 
                block.blockhash.as_deref().unwrap_or("unknown"), 
                block.slot);
            
            if let Err(e) = store_clone.store_block(block.clone()) {
                error!("Failed to store block: {}", e);
            }
        }
    });
    
    let node_handle = tokio::spawn(async move {
        if let Err(e) = node.start().await {
            error!("Node error: {}", e);
        }
    });
    
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");
    let _ = shutdown_tx.send(()).await;
    
    let _ = tokio::time::timeout(Duration::from_secs(5), node_handle).await;
    
    info!("✅ Local generator shutdown complete");
    Ok(())
} 
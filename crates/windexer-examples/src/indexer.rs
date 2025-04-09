use {
    anyhow::Result,
    clap::Parser,
    solana_sdk::{
        pubkey::Pubkey,
        signature::Keypair,
    },
    std::{
        path::PathBuf,
        sync::{Arc, Mutex},
        time::{Duration, Instant, SystemTime},
    },
    tokio::{
        time::interval,
    },
    tracing::{error, info},
    tracing_subscriber::{EnvFilter, fmt::format::FmtSpan},
    windexer_common::{
        config::NodeConfig,
        crypto::SerializableKeypair,
    },
    windexer_network::Node,
    windexer_store::{
        Store,
        StoreConfig,
    },
    windexer_metrics::metrics::metrics::*,
};

use warp::Filter;
use serde_json::json;

#[derive(Parser, Debug)]
#[clap(
    version, 
    about = "wIndexer data processor",
    long_about = "Connects to the wIndexer network, processes data from Geyser plugin, and stores it"
)]
struct Args {
    #[clap(short, long)]
    index: u16,
    
    #[clap(short, long, default_value = "9000")]
    base_port: u16,
    
    #[clap(long, value_delimiter = ',')]
    bootstrap_peers: Vec<String>,

    #[clap(long, default_value = "http://localhost:8899")]
    solana_rpc: String,
    
    #[clap(long, default_value = "./data")]
    data_dir: String,
    
    #[clap(long, default_value = "accounts,transactions,blocks")]
    index_types: String,
    
    #[clap(long)]
    log_level: Option<String>,
    
    #[clap(long, default_value = "30")]
    metrics_interval_seconds: u64,
}

// Struct to track indexing metrics
struct IndexingMetrics {
    start_time: Instant,
    accounts_processed: usize,
    transactions_processed: usize,
    blocks_processed: usize,
    last_processed_slot: u64,
    accounts_per_second: f64,
    transactions_per_second: f64,
    blocks_per_second: f64,
}

impl Default for IndexingMetrics {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            accounts_processed: 0,
            transactions_processed: 0,
            blocks_processed: 0,
            last_processed_slot: 0,
            accounts_per_second: 0.0,
            transactions_per_second: 0.0,
            blocks_per_second: 0.0,
        }
    }
}

impl IndexingMetrics {
    fn update_rates(&mut self) {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.accounts_per_second = self.accounts_processed as f64 / elapsed;
            self.transactions_per_second = self.transactions_processed as f64 / elapsed;
            self.blocks_per_second = self.blocks_processed as f64 / elapsed;
        }
    }
    
    fn log_metrics(&self) {
        info!(
            "📊 Indexing metrics: {} accounts ({:.2}/s), {} txs ({:.2}/s), {} blocks ({:.2}/s), last slot: {}",
            self.accounts_processed,
            self.accounts_per_second,
            self.transactions_processed,
            self.transactions_per_second,
            self.blocks_processed,
            self.blocks_per_second,
            self.last_processed_slot,
        );
    }
}

async fn start_api_server(store: Arc<Store>, port: u16) -> Result<()> {
    let store_clone = store.clone();
    let metrics_route = warp::path("api")
        .and(warp::path("status"))
        .map(move || {
            info!("Received API status request");
            let response = warp::reply::json(&json!({
                "status": "running",
                "accounts": store_clone.account_count(),
                "transactions": store_clone.transaction_count(),
                "blocks": store_clone.block_count(),
                "timestamp": SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
            }));
            info!("Returning API status response");
            response
        });
    
    let store_clone = store.clone();
    let accounts_route = warp::path("api")
        .and(warp::path("accounts"))
        .map(move || {
            info!("Received API accounts request");
            let accounts = store_clone.get_recent_accounts(10);
            info!("Returning {} accounts", accounts.len());
            warp::reply::json(&accounts)
        });
    
    let store_clone = store.clone();
    let transactions_route = warp::path("api")
        .and(warp::path("transactions"))
        .map(move || {
            info!("Received API transactions request");
            let txs = store_clone.get_recent_transactions(10);
            info!("Returning {} transactions", txs.len());
            warp::reply::json(&txs)
        });
    
    // Add a health check route
    let health_route = warp::path("health")
        .map(|| {
            info!("Received health check request");
            warp::reply::json(&json!({"status": "ok"}))
        });
    
    // Add a root route
    let root_route = warp::path::end()
        .map(|| {
            info!("Received root request");
            warp::reply::json(&json!({"message": "wIndexer API is running"}))
        });
    
    let routes = metrics_route
        .or(accounts_route)
        .or(transactions_route)
        .or(health_route)
        .or(root_route);
    
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    info!("🌐 Starting API server on {}", addr);
    
    // Start the server in a separate thread
    tokio::spawn(async move {
        info!("API server now listening on {}", addr);
        warp::serve(routes).run(addr).await;
    });
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    let log_level = args.log_level.unwrap_or_else(|| "info".to_string());
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new(format!(
                "{}={}",
                format!("indexer_{}", args.index),
                log_level
            ))
        });

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();

    let port = args.base_port + args.index;
    let rpc_port = port + 1000;
    let metrics_port = 9100 + args.index;
    
    info!("🔧 Configuring indexer {} with ports:", args.index);
    info!("   P2P: {}", port);
    info!("   RPC: {}", rpc_port);
    info!("   Metrics: {}", metrics_port);

    let index_accounts = args.index_types.contains("accounts");
    let index_transactions = args.index_types.contains("transactions");
    let index_blocks = args.index_types.contains("blocks");
    
    info!("📊 Indexing: {}{}{}",
        if index_accounts { "accounts " } else { "" },
        if index_transactions { "transactions " } else { "" },
        if index_blocks { "blocks" } else { "" }
    );

    let node_config = NodeConfig {
        node_id: format!("indexer_{}", args.index),
        listen_addr: format!("127.0.0.1:{}", port).parse()?,
        rpc_addr: format!("127.0.0.1:{}", rpc_port).parse()?,
        bootstrap_peers: args.bootstrap_peers,
        data_dir: format!("{}/indexer_{}", args.data_dir, args.index),
        solana_rpc_url: args.solana_rpc,
        keypair: SerializableKeypair::new(&Keypair::new()),
        geyser_plugin_config: None,
        metrics_addr: Some(format!("127.0.0.1:{}", metrics_port).parse()?),
    };

    info!("🚀 Starting wIndexer node");
    let (mut node, shutdown_tx) = Node::create_simple(node_config).await?;
    
    let store_path = PathBuf::from(format!("{}/indexer_{}/store", args.data_dir, args.index));
    std::fs::create_dir_all(&store_path)?;
    
    let store_config = StoreConfig {
        path: store_path,
        max_open_files: 1000,
        cache_capacity: 100 * 1024 * 1024, // 100 MB
    };
    
    info!("💾 Initializing storage");
    let store = Store::open(store_config)?;
    let store = Arc::new(store);
    
    let api_port = args.base_port;
    start_api_server(store.clone(), api_port).await?;
    
    let metrics = Arc::new(Mutex::new(IndexingMetrics::default()));
    
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

    
    let node_handle = tokio::spawn(async move {
        if let Err(e) = node.start().await {
            error!("Node error: {}", e);
        }
    });
    
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");
    let _ = shutdown_tx.send(()).await;
    
    let _ = tokio::time::timeout(Duration::from_secs(5), node_handle).await;
    
    info!("✅ Indexer shutdown complete");
    Ok(())
} 
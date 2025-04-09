pub mod metrics {
    pub mod metrics {
        use lazy_static::lazy_static;
        use prometheus::{register_int_counter, IntCounter};
        
        lazy_static! {
            pub static ref ACCOUNTS_PROCESSED: IntCounter =
                register_int_counter!("windexer_accounts_processed", "Number of accounts processed").unwrap();
            
            pub static ref TRANSACTIONS_PROCESSED: IntCounter =
                register_int_counter!("windexer_transactions_processed", "Number of transactions processed").unwrap();
            
            pub static ref BLOCKS_PROCESSED: IntCounter =
                register_int_counter!("windexer_blocks_processed", "Number of blocks processed").unwrap();
        }
    }
    
    pub mod wmetrics {
        use anyhow::Result;
        use prometheus::{Encoder, TextEncoder};
        use std::net::SocketAddr;
        use tokio::sync::oneshot;
        use warp::Filter;
        
        pub async fn start_metrics_server(addr: SocketAddr) -> Result<oneshot::Sender<()>> {
            let (shutdown_tx, shutdown_rx) = oneshot::channel();
            
            let metrics_route = warp::path!("metrics").map(|| {
                let encoder = TextEncoder::new();
                let metric_families = prometheus::gather();
                let mut buffer = Vec::new();
                encoder.encode(&metric_families, &mut buffer).unwrap();
                warp::reply::with_header(buffer, "Content-Type", encoder.format_type())
            });
            
            let routes = metrics_route;
            
            tokio::spawn(async move {
                let (_, server) = warp::serve(routes)
                    .bind_with_graceful_shutdown(addr, async {
                        shutdown_rx.await.ok();
                    });
                server.await;
            });
            
            Ok(shutdown_tx)
        }
    }
} 
use std::net::SocketAddr;
use std::str::FromStr;
use anyhow::Result;
use windexer_api::{ApiServer, ApiConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let port = std::env::var("API_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap_or(3000);
    
    let log_level = std::env::var("LOG_LEVEL")
        .unwrap_or_else(|_| "info".to_string());
    
    let config = ApiConfig {
        bind_addr: SocketAddr::from_str(&format!("0.0.0.0:{}", port))?,
        log_level,
    };
    
    let server = ApiServer::new(config);
    server.start().await?;
    
    Ok(())
} 
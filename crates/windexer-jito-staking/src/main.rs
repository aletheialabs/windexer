// crates/windexer-jito-staking/src/main.rs

use windexer_jito_staking::{JitoStakingService, StakingConfig};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup minimal logging
    tracing_subscriber::fmt::init();
    
    // Create default config
    let config = StakingConfig {
        min_stake: 1_000_000_000,
        min_operators: 3,
        consensus_threshold: 0.66,
        reward_rate: 0.10,
        distribution_interval: std::time::Duration::from_secs(86400),
        slash_threshold: 0.95,
        min_uptime: 0.98,
    };
    
    // Initialize service
    let staking_service = JitoStakingService::new(config);
    
    // Start service
    staking_service.start().await?;
    
    // Keep running until Ctrl+C
    tokio::signal::ctrl_c().await?;
    println!("Shutting down...");
    
    Ok(())
}
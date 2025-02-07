// crates/windexer-jito-staking/src/main.rs

use anyhow::{Result, Context};
use tracing::{info, error};
use windexer_jito_staking::{JitoStakingService, StakingConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting Jito staking service...");

    let config = load_config().context("Failed to load configuration")?;

    let staking_service = JitoStakingService::new(config);
    
    match staking_service.start().await {
        Ok(()) => {
            info!("Jito staking service started successfully");
            tokio::signal::ctrl_c().await?;
            info!("Shutting down staking service...");
        }
        Err(e) => {
            error!("Failed to start staking service: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn load_config() -> Result<StakingConfig> {
    Ok(StakingConfig {
        min_stake: 1000,
        commission_bps: 500,
        min_delegation_period: Duration::from_secs(86400),
        max_operator_stake: 1_000_000_000_000,
        min_operators: 4,
        consensus_threshold: 0.67,
        reward_rate: 0.10,
        distribution_interval: Duration::from_secs(86400),
        slash_threshold: 0.95,
        min_uptime: 0.99,
    })
}
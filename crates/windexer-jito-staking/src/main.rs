use windexer_jito_staking::StakingConfig;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let config = StakingConfig {
        min_stake: 1000,
        reward_rate: 0.05,
        slash_percentage: 0.1,
        reward_window: Duration::from_secs(86400),
    };

    println!("Staking service started with config: {:?}", config);
} 
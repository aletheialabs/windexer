//! wIndexer AVS (Actively Validated Service) binary using Cambrian CLI

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::{info, error};
use tracing_subscriber::FmtSubscriber;
use windexer_jito_staking::cambrian::{CambrianConfig, CambrianService};

#[derive(Parser, Debug)]
#[command(
    name = "windexer-avs",
    about = "wIndexer Actively Validated Service (AVS) using Cambrian CLI",
    version
)]
struct Cli {
    /// IP address to bind to
    #[arg(long, default_value = "127.0.0.1")]
    ip: String,
    
    /// HTTP port to bind to
    #[arg(long, default_value = "8080")]
    http_port: u16,
    
    /// WebSocket port to bind to
    #[arg(long, default_value = "8081")]
    ws_port: u16,
    
    /// Admin keypair path
    #[arg(long, default_value = "./admin-keypair.json")]
    admin_keypair: PathBuf,
    
    /// Solana API URL
    #[arg(long, default_value = "http://localhost:8899")]
    solana_api_url: String,
    
    /// Command to execute (init, run, register-operator, run-payload)
    #[command(subcommand)]
    command: CambrianCommand,
}

#[derive(Parser, Debug)]
enum CambrianCommand {
    /// Initialize a new AVS
    Init {
        /// AVS name
        #[arg(long)]
        name: Option<String>,
    },
    
    /// Run the AVS node
    Run,
    
    /// Register as an operator
    RegisterOperator {
        /// Operator keypair
        #[arg(long)]
        keypair: PathBuf,
        
        /// Stake amount in SOL
        #[arg(long, default_value = "1.0")]
        stake: f64,
    },
    
    /// Run a payload
    RunPayload {
        /// Payload image
        #[arg(long)]
        image: String,
        
        /// PoA pubkey
        #[arg(long)]
        poa: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger with more detail for debug
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
    
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Setup configuration
    let config = CambrianConfig {
        avs_ip: cli.ip,
        avs_http_port: cli.http_port,
        avs_ws_port: cli.ws_port,
        admin_keypair_path: cli.admin_keypair,
        solana_api_url: cli.solana_api_url.clone(),
        solana_ws_url: cli.solana_api_url.replace("http", "ws"),
        ccp_name: match &cli.command {
            CambrianCommand::Init { name } => name.clone().unwrap_or_else(|| format!("windexer-avs-{}", chrono::Utc::now().timestamp())),
            _ => "windexer-avs".to_string(),
        },
        proposal_storage_key: format!("windexer-storage-{}", chrono::Utc::now().timestamp()),
        storage_space: 1024 * 1024, // 1 MB
        consensus_threshold: 0.66,
        stake_threshold: 1_000_000_000, // 1 SOL in lamports
    };
    
    // Create Cambrian service
    let service = CambrianService::new(config);
    
    // Execute the command
    match cli.command {
        CambrianCommand::Init { .. } => {
            info!("Initializing AVS");
            match service.initialize_avs().await {
                Ok(poa_pubkey) => {
                    info!("AVS initialized with PoA pubkey: {}", poa_pubkey);
                },
                Err(e) => {
                    error!("Failed to initialize AVS: {}", e);
                    return Err(e);
                }
            }
        },
        
        CambrianCommand::Run => {
            info!("Starting AVS");
            match service.start_avs().await {
                Ok(_) => {
                    info!("AVS server started, press Ctrl+C to stop");
                    
                    // Wait for Ctrl+C
                    match tokio::signal::ctrl_c().await {
                        Ok(_) => info!("Received Ctrl+C, shutting down"),
                        Err(e) => error!("Error waiting for Ctrl+C: {}", e),
                    }
                },
                Err(e) => {
                    error!("Failed to start AVS server: {}", e);
                    return Err(e);
                }
            }
        },
        
        CambrianCommand::RegisterOperator { keypair: _, stake } => {
            info!("Registering operator with stake {} SOL", stake);
            // This would call the Cambrian CLI to register an operator
            // For now, we'll just print a message
            info!("Operator registration would be handled by Cambrian CLI");
        },
        
        CambrianCommand::RunPayload { image, poa: _ } => {
            info!("Running payload: {}", image);
            // This would call the Cambrian CLI to run a payload
            // For now, we'll just print a message
            info!("Payload execution would be handled by Cambrian CLI");
        },
    }
    
    Ok(())
} 
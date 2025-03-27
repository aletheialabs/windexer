//! Payload manager for Cambrian integration using Cambrian CLI

use super::{CambrianConfig, PoAState};
use anyhow::{Result, anyhow};
use std::{
    fs,
    path::PathBuf,
    process::Command,
};
use tracing::{info, error};

/// Payload manager using Cambrian CLI
pub struct PayloadManager {
    config: CambrianConfig,
}

impl PayloadManager {
    /// Create a new payload manager
    pub fn new(config: CambrianConfig) -> Self {
        Self {
            config,
        }
    }
    
    /// Run a payload container using Cambrian CLI
    pub async fn run_payload(
        &self,
        payload_image: &str,
        poa_state: &PoAState,
    ) -> Result<String> {
        info!("Running payload container via Cambrian CLI: {}", payload_image);
        
        // Use Cambrian CLI to run payload
        let output = Command::new("cambrian")
            .args(&[
                "payload",
                "run",
                "--keypair", self.config.admin_keypair_path.to_str().unwrap(),
                "--image", payload_image,
                "--poa", &poa_state.pubkey.to_string(),
                "--url", &self.config.solana_api_url,
                "--output", "proposal.json",
            ])
            .output()?;
        
        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            error!("Cambrian CLI error: {}", error_message);
            return Err(anyhow!("Failed to run payload: {}", error_message));
        }
        
        // Log the output
        let stdout = String::from_utf8_lossy(&output.stdout);
        info!("Cambrian CLI output: {}", stdout);
        
        // Verify proposal.json was created
        if !PathBuf::from("proposal.json").exists() {
            return Err(anyhow!("Proposal file not created"));
        }
        
        info!("Payload executed successfully, proposal file created");
        Ok("proposal.json".to_string())
    }
    
    /// Build a payload container image using Cambrian CLI
    pub async fn build_payload_image(&self, path: &PathBuf) -> Result<String> {
        info!("Building payload image from path via Cambrian CLI: {:?}", path);
        
        // Ensure path exists
        if !path.exists() {
            return Err(anyhow!("Path does not exist: {:?}", path));
        }
        
        // Generate a unique tag for the image
        let tag = format!("windexer-payload-{}", chrono::Utc::now().timestamp());
        
        // Use Cambrian CLI to build payload
        let output = Command::new("cambrian")
            .args(&[
                "payload",
                "build",
                "--path", path.to_str().unwrap(),
                "--tag", &tag,
            ])
            .output()?;
        
        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            error!("Cambrian CLI error: {}", error_message);
            return Err(anyhow!("Failed to build payload: {}", error_message));
        }
        
        info!("Payload image built with tag: {}", tag);
        Ok(tag)
    }
} 
//! AVS (Actively Validated Services) manager implementation using Cambrian CLI

use super::{CambrianConfig, PoAState};
use anyhow::{Result, anyhow};
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
};
use std::{
    process::Command,
    str::FromStr,
};
use tracing::{info, warn, error};

/// AVS Manager handling the Actively Validated Service through Cambrian CLI
pub struct AvsManager {
    config: CambrianConfig,
}

impl AvsManager {
    /// Create a new AVS manager
    pub fn new(config: CambrianConfig) -> Self {
        Self {
            config,
        }
    }
    
    /// Initialize the AVS on-chain using Cambrian CLI
    pub async fn initialize_avs(&self) -> Result<Pubkey> {
        info!("Initializing AVS on-chain with Cambrian CLI");
        
        // Use Cambrian CLI to initialize AVS
        let output = Command::new("cambrian")
            .args(&[
                "avs",
                "init",
                "--keypair", self.config.admin_keypair_path.to_str().unwrap(),
                "--name", &self.config.ccp_name,
                "--url", &self.config.solana_api_url,
            ])
            .output()?;
        
        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            error!("Cambrian CLI error: {}", error_message);
            return Err(anyhow!("Failed to initialize AVS: {}", error_message));
        }
        
        // Parse the output to extract the PoA pubkey
        let stdout = String::from_utf8_lossy(&output.stdout);
        info!("Cambrian CLI output: {}", stdout);
        
        // Extract the PoA pubkey from the output (implementation depends on CLI output format)
        let poa_pubkey_str = extract_poa_pubkey_from_output(&stdout)?;
        let poa_pubkey = Pubkey::from_str(poa_pubkey_str)?;
        
        info!("AVS initialized with PoA pubkey: {}", poa_pubkey);
        Ok(poa_pubkey)
    }
    
    // Add this method to the AvsManager implementation
pub async fn start_avs(&self) -> Result<()> {
    info!("Starting mock AVS server at {}:{}", self.config.avs_ip, self.config.avs_http_port);
    
    // Create a simple HTTP server for the API
    let addr = format!("{}:{}", self.config.avs_ip, self.config.avs_http_port)
        .parse()
        .expect("Failed to parse address");
    
    // Set up a simple status endpoint and payload execution endpoint
    let make_service = hyper::service::make_service_fn(|_conn| async {
        Ok::<_, hyper::Error>(hyper::service::service_fn(|req| async move {
            let uri = req.uri().path();
            
            match uri {
                "/api/status" => {
                    // Status endpoint
                    Ok::<_, hyper::Error>(hyper::Response::builder()
                        .status(200)
                        .header("Content-Type", "application/json")
                        .body(hyper::Body::from("{\"status\":\"ok\"}"))
                        .unwrap())
                },
                "/api/payload/run" => {
                    // Mock payload execution
                    // Parse the request body
                    let whole_body = hyper::body::to_bytes(req.into_body()).await?;
                    let payload: serde_json::Value = serde_json::from_slice(&whole_body)
                        .unwrap_or(serde_json::json!({}));
                    
                    // Log the payload request
                    info!("Received payload execution request: {:?}", payload);
                    
                    // Return a mock success response
                    Ok(hyper::Response::builder()
                        .status(200)
                        .header("Content-Type", "application/json")
                        .body(hyper::Body::from(
                            "{\"status\":\"success\",\"signature\":\"mock-signature-12345\"}"
                        ))
                        .unwrap())
                },
                _ => {
                    // 404 for any other path
                    Ok(hyper::Response::builder()
                        .status(404)
                        .body(hyper::Body::from("Not found"))
                        .unwrap())
                }
            }
        }))
    });
    
    let server = hyper::Server::bind(&addr).serve(make_service);
    info!("AVS server started, listening on http://{}", addr);
    
    // Run the server
    if let Err(e) = server.await {
        error!("Server error: {}", e);
        return Err(anyhow::anyhow!("Server error: {}", e));
    }
        
    Ok(())
}
    
    /// Submit a proposal to the PoA program using Cambrian CLI
    pub async fn submit_proposal(
        &self,
        proposal_file_path: &str,
        poa_state: &PoAState,
    ) -> Result<Signature> {
        info!("Submitting proposal to PoA program using Cambrian CLI: {}", poa_state.pubkey);
        
        // Use Cambrian CLI to submit proposal
        let output = Command::new("cambrian")
            .args(&[
                "proposal",
                "submit",
                "--keypair", self.config.admin_keypair_path.to_str().unwrap(),
                "--poa", &poa_state.pubkey.to_string(),
                "--proposal", proposal_file_path,
                "--url", &self.config.solana_api_url,
            ])
            .output()?;
        
        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            error!("Cambrian CLI error: {}", error_message);
            return Err(anyhow!("Failed to submit proposal: {}", error_message));
        }
        
        // Parse the output to extract the transaction signature
        let stdout = String::from_utf8_lossy(&output.stdout);
        info!("Cambrian CLI output: {}", stdout);
        
        // Extract the signature from the output (implementation depends on CLI output format)
        let signature_str = extract_signature_from_output(&stdout)?;
        let signature = Signature::from_str(signature_str)?;
        
        info!("Proposal submitted with signature: {}", signature);
        Ok(signature)
    }
}

// Helper function to extract PoA pubkey from Cambrian CLI output
fn extract_poa_pubkey_from_output(output: &str) -> Result<&str> {
    // This implementation depends on the exact format of Cambrian CLI output
    // For now, we'll use a simplified example that looks for a pubkey pattern
    
    for line in output.lines() {
        if line.contains("PoA pubkey:") {
            if let Some(pubkey) = line.split("PoA pubkey:").nth(1).map(|s| s.trim()) {
                return Ok(pubkey);
            }
        }
    }
    
    Err(anyhow!("Could not find PoA pubkey in CLI output"))
}

// Helper function to extract signature from Cambrian CLI output
fn extract_signature_from_output(output: &str) -> Result<&str> {
    // This implementation depends on the exact format of Cambrian CLI output
    // For now, we'll use a simplified example that looks for a signature pattern
    
    for line in output.lines() {
        if line.contains("Signature:") {
            if let Some(signature) = line.split("Signature:").nth(1).map(|s| s.trim()) {
                return Ok(signature);
            }
        }
    }
    
    Err(anyhow!("Could not find signature in CLI output"))
} 
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use anyhow::Result;

use crate::account_endpoints::AccountData;
use crate::helius::HeliusClient;

pub struct AccountDataManager {
    helius_client: Arc<HeliusClient>,
    
    cache: Arc<RwLock<HashMap<String, AccountData>>>,
    
    update_sender: broadcast::Sender<AccountData>,
    
    initialized: Arc<RwLock<bool>>,
}

impl AccountDataManager {
    pub fn new(helius_client: Arc<HeliusClient>) -> Self {
        let (tx, _) = broadcast::channel(10000); // Buffer for 10,000 account updates
        
        Self {
            helius_client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            update_sender: tx,
            initialized: Arc::new(RwLock::new(false)),
        }
    }
    
    pub async fn initialize(&self) -> Result<()> {
        let mut initialized = self.initialized.write().await;
        
        if *initialized {
            return Ok(());
        }
        
        if let Err(e) = self.helius_client.connect_websocket().await {
            tracing::warn!("Failed to connect to Helius WebSocket: {}", e);
            // Continue even if WebSocket connection fails
        }
        
        *initialized = true;
        
        Ok(())
    }
    
    /// Subscribe to a Solana program for account updates
    pub async fn subscribe_to_program(&self, program_id: &str) -> Result<()> {
        self.helius_client.subscribe_program_updates(program_id).await
    }
    
    /// Subscribe to a specific account
    pub async fn subscribe_to_account(&self, pubkey: &str) -> Result<()> {
        self.helius_client.subscribe_account_updates(pubkey).await
    }
    
    /// Get account data from cache
    pub async fn get_account(&self, pubkey: &str) -> Result<AccountData> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(account) = cache.get(pubkey) {
                return Ok(account.clone());
            }
        }
        
        // Not in cache, fetch from Helius
        let response = self.helius_client.get_account_info(pubkey).await?;
        
        tracing::debug!("Helius account response: {:?}", response);
        
        // Parse the response
        let result = response.get("result").ok_or_else(|| anyhow::anyhow!("Missing result field in response"))?;
        let context = result.get("context").ok_or_else(|| anyhow::anyhow!("Missing context field in result"))?;
        let value = result.get("value").ok_or_else(|| anyhow::anyhow!("Missing value field in result"))?;
        
        let slot = context.get("slot").and_then(|s| s.as_u64()).unwrap_or(0) as u64;
        
        // Handle null value (account not found)
        if value.is_null() {
            return Err(anyhow::anyhow!("Account not found"));
        }
        
        // Extract account data
        let lamports = value.get("lamports").and_then(|l| l.as_u64()).unwrap_or(0);
        let owner = value.get("owner").and_then(|o| o.as_str()).unwrap_or("").to_string();
        let executable = value.get("executable").and_then(|e| e.as_bool()).unwrap_or(false);
        let rent_epoch = value.get("rentEpoch").and_then(|r| r.as_u64()).unwrap_or(0);
        
        // Data might be encoded as base64 or array of bytes
        let data_base64 = if let Some(data) = value.get("data") {
            if data.is_array() && data.as_array().unwrap().len() >= 2 {
                let data_array = data.as_array().unwrap();
                Some(data_array[0].as_str().unwrap_or("").to_string())
            } else {
                None
            }
        } else {
            None
        };
        
        let data = Vec::new(); // We'd need to decode the base64 data if needed
        
        let account = AccountData {
            pubkey: pubkey.to_string(),
            lamports,
            owner,
            executable,
            rent_epoch,
            data,
            data_base64,
            slot,
            updated_at: chrono::Utc::now().timestamp(),
        };
        
        // Update cache
        let mut cache = self.cache.write().await;
        cache.insert(pubkey.to_string(), account.clone());
        
        Ok(account)
    }
    
    /// Get accounts by program ID
    pub async fn get_accounts_by_program(&self, program_id: &str, limit: usize) -> Result<Vec<AccountData>> {
        // For now, return accounts from our cache that match the program
        // In a real implementation, we would use getProgramAccounts from Helius
        let cache = self.cache.read().await;
        let mut matching_accounts = Vec::new();
        
        for account in cache.values() {
            if account.owner == program_id {
                matching_accounts.push(account.clone());
                if matching_accounts.len() >= limit {
                    break;
                }
            }
        }
        
        Ok(matching_accounts)
    }
    
    /// Get a subscription to account updates
    pub fn subscribe(&self) -> broadcast::Receiver<AccountData> {
        self.update_sender.subscribe()
    }
}
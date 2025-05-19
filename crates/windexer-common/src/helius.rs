use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use crate::types::helius::{
    AccountData,
    BlockData,
    TransactionData,
};

/// Helius RPC endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeliusConfig {
    /// API key for Helius
    pub api_key: String,
    /// Network to connect to (mainnet, devnet, testnet)
    pub network: String,
    /// WebSocket endpoint
    pub ws_endpoint: Option<String>,
    /// HTTP endpoint
    pub http_endpoint: Option<String>,
}

impl Default for HeliusConfig {
    fn default() -> Self {
        Self {
            api_key: "".to_string(),
            network: "mainnet".to_string(),
            ws_endpoint: None,
            http_endpoint: None,
        }
    }
}

/// Helius API client for Solana blockchain data
#[derive(Debug, Clone)]
pub struct HeliusClient {
    /// Configuration
    config: HeliusConfig,
    /// HTTP client
    client: reqwest::Client,
    /// WebSocket connection (if established)
    ws_connection: Arc<RwLock<Option<tokio::sync::mpsc::Sender<Message>>>>,
    /// Account update channel
    account_updates: broadcast::Sender<AccountData>,
    /// Transaction update channel
    transaction_updates: broadcast::Sender<TransactionData>,
    /// Block update channel
    block_updates: broadcast::Sender<BlockData>,
}

impl HeliusClient {
    /// Create a new Helius client
    pub fn new(config: HeliusConfig) -> Self {
        let (account_tx, _) = broadcast::channel(1000);
        let (tx_tx, _) = broadcast::channel(1000);
        let (block_tx, _) = broadcast::channel(1000);
        
        Self {
            config,
            client: reqwest::Client::new(),
            ws_connection: Arc::new(RwLock::new(None)),
            account_updates: account_tx,
            transaction_updates: tx_tx,
            block_updates: block_tx,
        }
    }
    
    /// Create a new Helius client with just an API key
    pub fn new_with_key(api_key: &str) -> Self {
        Self::new(HeliusConfig {
            api_key: api_key.to_string(),
            ..Default::default()
        })
    }
    
    /// Get the base URL for HTTP requests
    fn get_base_url(&self) -> String {
        if let Some(endpoint) = &self.config.http_endpoint {
            format!("{}?api-key={}", endpoint, self.config.api_key)
        } else {
            format!("https://{}.helius-rpc.com/?api-key={}", 
                self.config.network, 
                self.config.api_key)
        }
    }
    
    /// Get the WebSocket URL
    fn get_ws_url(&self) -> String {
        if let Some(endpoint) = &self.config.ws_endpoint {
            format!("{}?api-key={}", endpoint, self.config.api_key)
        } else {
            format!("wss://{}.helius-rpc.com/v0/ws?api-key={}", 
                self.config.network, 
                self.config.api_key)
        }
    }
    
    /// Connect to the Helius WebSocket endpoint
    pub async fn connect_websocket(&self) -> Result<()> {
        let ws_url = self.get_ws_url();
        
        tracing::info!("Connecting to Helius WebSocket at {}", ws_url);
        
        let (ws_stream, _) = connect_async(ws_url).await
            .map_err(|e| anyhow!("Failed to connect to WebSocket: {}", e))?;
        
        tracing::info!("Connected to Helius WebSocket");
        
        // Split the WebSocket stream
        let (mut write, mut read) = ws_stream.split();
        
        // Create a channel for sending messages to the WebSocket
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        
        // Store the sender
        {
            let mut connection = self.ws_connection.write().await;
            *connection = Some(tx);
        }
        
        // Create channel clones for the task
        let account_updates = self.account_updates.clone();
        let transaction_updates = self.transaction_updates.clone();
        let block_updates = self.block_updates.clone();
        
        // Spawn a task to forward messages from the channel to the WebSocket
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(e) = write.send(message).await {
                    tracing::error!("Failed to send WebSocket message: {}", e);
                    break;
                }
            }
            tracing::info!("WebSocket sender task terminated");
        });
        
        // Spawn a task to process incoming WebSocket messages
        tokio::spawn(async move {
            while let Some(message_result) = read.next().await {
                match message_result {
                    Ok(message) => {
                        if let Message::Text(text) = message {
                            match serde_json::from_str::<serde_json::Value>(&text) {
                                Ok(json) => {
                                    if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
                                        match method {
                                            "accountNotification" => {
                                                // Process account update
                                                if let Some(account) = parse_account_update(&json) {
                                                    let _ = account_updates.send(account);
                                                }
                                            },
                                            "signatureNotification" => {
                                                // Process transaction update
                                                if let Some(tx) = parse_transaction_update(&json) {
                                                    let _ = transaction_updates.send(tx);
                                                }
                                            },
                                            "slotNotification" => {
                                                // Process block update
                                                if let Some(block) = parse_block_update(&json) {
                                                    let _ = block_updates.send(block);
                                                }
                                            },
                                            _ => {
                                                tracing::debug!("Received unhandled WebSocket message: {}", method);
                                            }
                                        }
                                    }
                                },
                                Err(e) => {
                                    tracing::error!("Failed to parse WebSocket message: {}", e);
                                }
                            }
                        }
                    },
                    Err(e) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                }
            }
            tracing::info!("WebSocket receiver task terminated");
        });
        
        Ok(())
    }
    
    /// Send a JSON-RPC request to Helius
    pub async fn send_rpc_request<T: Serialize>(&self, request: &T) -> Result<serde_json::Value> {
        let url = self.get_base_url();
        
        let response = self.client.post(&url)
            .json(request)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;
        
        if !response.status().is_success() {
            return Err(anyhow!("HTTP error: {}", response.status()));
        }
        
        let json = response.json::<serde_json::Value>().await
            .map_err(|e| anyhow!("Failed to parse JSON response: {}", e))?;
        
        if let Some(error) = json.get("error") {
            return Err(anyhow!("RPC error: {}", error));
        }
        
        Ok(json)
    }
    
    /// Send a WebSocket subscription request
    pub async fn send_subscription(&self, method: &str, params: Vec<serde_json::Value>) -> Result<()> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });
        
        let connection = self.ws_connection.read().await;
        if let Some(sender) = &*connection {
            sender.send(Message::Text(request.to_string())).await
                .map_err(|e| anyhow!("Failed to send subscription: {}", e))?;
            Ok(())
        } else {
            Err(anyhow!("WebSocket not connected"))
        }
    }
    
    /// Subscribe to account updates
    pub async fn subscribe_account(&self, pubkey: &str) -> Result<()> {
        self.send_subscription(
            "accountSubscribe",
            vec![
                serde_json::Value::String(pubkey.to_string()),
                serde_json::json!({
                    "commitment": "confirmed",
                    "encoding": "base64"
                })
            ]
        ).await
    }
    
    /// Subscribe to transaction updates for a specific account
    pub async fn subscribe_signatures(&self, pubkey: &str) -> Result<()> {
        self.send_subscription(
            "signatureSubscribe",
            vec![
                serde_json::json!({
                    "mentions": [pubkey]
                }),
                serde_json::json!({
                    "commitment": "confirmed",
                    "enableReceivedNotification": true
                })
            ]
        ).await
    }
    
    /// Subscribe to program updates
    pub async fn subscribe_program(&self, program_id: &str) -> Result<()> {
        self.send_subscription(
            "programSubscribe",
            vec![
                serde_json::Value::String(program_id.to_string()),
                serde_json::json!({
                    "commitment": "confirmed",
                    "encoding": "base64"
                })
            ]
        ).await
    }
    
    /// Subscribe to block/slot updates
    pub async fn subscribe_slots(&self) -> Result<()> {
        self.send_subscription(
            "slotSubscribe",
            vec![]
        ).await
    }
    
    /// Get account data
    pub async fn get_account(&self, pubkey: &str) -> Result<AccountData> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [
                pubkey,
                {"encoding": "base64"}
            ]
        });
        
        let response = self.send_rpc_request(&request).await?;
        
        parse_account_response(pubkey, &response)
            .ok_or_else(|| anyhow!("Failed to parse account data"))
    }
    
    /// Get transaction data
    pub async fn get_transaction(&self, signature: &str) -> Result<TransactionData> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTransaction",
            "params": [
                signature,
                {"encoding": "json", "maxSupportedTransactionVersion": 0}
            ]
        });
        
        let response = self.send_rpc_request(&request).await?;
        
        parse_transaction_response(signature, &response)
            .ok_or_else(|| anyhow!("Failed to parse transaction data"))
    }
    
    /// Get block data
    pub async fn get_block(&self, slot: u64) -> Result<BlockData> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBlock",
            "params": [
                slot,
                {"encoding": "json", "transactionDetails": "signatures", "maxSupportedTransactionVersion": 0}
            ]
        });
        
        let response = self.send_rpc_request(&request).await?;
        
        parse_block_response(slot, &response)
            .ok_or_else(|| anyhow!("Failed to parse block data"))
    }
    
    /// Get the latest block/slot
    pub async fn get_latest_slot(&self) -> Result<u64> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getSlot",
            "params": []
        });
        
        let response = self.send_rpc_request(&request).await?;
        
        response.get("result")
            .and_then(|slot| slot.as_u64())
            .ok_or_else(|| anyhow!("Failed to get latest slot"))
    }
    
    /// Get a subscription to account updates
    pub fn subscribe_account_updates(&self) -> broadcast::Receiver<AccountData> {
        self.account_updates.subscribe()
    }
    
    /// Get a subscription to transaction updates
    pub fn subscribe_transaction_updates(&self) -> broadcast::Receiver<TransactionData> {
        self.transaction_updates.subscribe()
    }
    
    /// Get a subscription to block updates
    pub fn subscribe_block_updates(&self) -> broadcast::Receiver<BlockData> {
        self.block_updates.subscribe()
    }
}

/// Parse an account update from a WebSocket notification
fn parse_account_update(json: &serde_json::Value) -> Option<AccountData> {
    if let Some(params) = json.get("params")?.as_array() {
        if params.len() >= 2 {
            if let Some(result) = params[1].get("result") {
                if let Some(value) = result.get("value") {
                    let pubkey = params[0].as_str()?.to_string();
                    let lamports = value.get("lamports")?.as_u64()?;
                    let owner = value.get("owner")?.as_str()?.to_string();
                    let executable = value.get("executable")?.as_bool()?;
                    let rent_epoch = value.get("rentEpoch")?.as_u64()?;
                    let data_base64 = value.get("data")?.as_array()?[0].as_str()?.to_string();
                    let data = base64::decode(&data_base64).ok()?;
                    let slot = result.get("context")?.get("slot")?.as_u64()?;
                    
                    return Some(AccountData {
                        pubkey,
                        lamports,
                        owner,
                        executable,
                        rent_epoch,
                        data,
                        slot,
                        write_version: 0,
                        updated_on: chrono::Utc::now().timestamp(),
                        is_startup: false,
                        transaction_signature: None,
                    });
                }
            }
        }
    }
    None
}

/// Parse a transaction update from a WebSocket notification
fn parse_transaction_update(json: &serde_json::Value) -> Option<TransactionData> {
    if let Some(params) = json.get("params")?.as_array() {
        if params.len() >= 2 {
            if let Some(result) = params[1].get("result") {
                let signature = params[0].as_str()?.to_string();
                let slot = result.get("context")?.get("slot")?.as_u64()?;
                let err = result.get("value")?.get("err").cloned();
                
                // In a real implementation, we would parse the complete transaction
                // For now, return a simplified version
                return Some(TransactionData {
                    signature,
                    slot,
                    err: err.is_some(),
                    status: if err.is_none() { 1 } else { 0 },
                    fee: 5000, // Placeholder
                    fee_payer: "11111111111111111111111111111111".to_string(), // Placeholder
                    recent_blockhash: "11111111111111111111111111111111".to_string(), // Placeholder
                    accounts: vec![], // Placeholder
                    log_messages: vec![], // Placeholder
                    block_time: Some(chrono::Utc::now().timestamp()),
                });
            }
        }
    }
    None
}

/// Parse a block update from a WebSocket notification
fn parse_block_update(json: &serde_json::Value) -> Option<BlockData> {
    if let Some(params) = json.get("params")?.as_array() {
        if params.len() >= 1 {
            let slot = params[0].as_u64()?;
            
            // In a real implementation, we would query for the complete block
            // For now, return a simplified version
            return Some(BlockData {
                slot,
                blockhash: format!("simulated_blockhash_{}", slot),
                parent_slot: slot.saturating_sub(1),
                parent_blockhash: format!("simulated_blockhash_{}", slot.saturating_sub(1)),
                block_time: Some(chrono::Utc::now().timestamp()),
                block_height: Some(slot),
                transaction_count: Some(0),
                status: Some(1), // Confirmed
                leader: None,
            });
        }
    }
    None
}

/// Parse an account response from a JSON-RPC call
fn parse_account_response(pubkey: &str, json: &serde_json::Value) -> Option<AccountData> {
    if let Some(result) = json.get("result") {
        if let Some(value) = result.get("value") {
            let lamports = value.get("lamports")?.as_u64()?;
            let owner = value.get("owner")?.as_str()?.to_string();
            let executable = value.get("executable")?.as_bool()?;
            let rent_epoch = value.get("rentEpoch")?.as_u64()?;
            let data_base64 = value.get("data")?.as_array()?[0].as_str()?.to_string();
            let data = base64::decode(&data_base64).ok()?;
            let slot = result.get("context")?.get("slot")?.as_u64()?;
            
            return Some(AccountData {
                pubkey: pubkey.to_string(),
                lamports,
                owner,
                executable,
                rent_epoch,
                data,
                slot,
                write_version: 0,
                updated_on: chrono::Utc::now().timestamp(),
                is_startup: false,
                transaction_signature: None,
            });
        }
    }
    None
}

/// Parse a transaction response from a JSON-RPC call
fn parse_transaction_response(signature: &str, json: &serde_json::Value) -> Option<TransactionData> {
    if let Some(result) = json.get("result") {
        let slot = result.get("slot")?.as_u64()?;
        let meta = result.get("meta")?;
        let err = meta.get("err");
        let fee = meta.get("fee")?.as_u64()?;
        let block_time = result.get("blockTime").and_then(|bt| bt.as_i64());
        
        // Extract log messages if available
        let log_messages = meta.get("logMessages").and_then(|logs| {
            logs.as_array().map(|arr| {
                arr.iter()
                   .filter_map(|log| log.as_str().map(|s| s.to_string()))
                   .collect()
            })
        }).unwrap_or_default();
        
        // In a real implementation, we would parse the complete transaction
        // For now, return a simplified version
        return Some(TransactionData {
            signature: signature.to_string(),
            slot,
            err: err.is_some(),
            status: if err.is_none() { 1 } else { 0 },
            fee,
            fee_payer: "11111111111111111111111111111111".to_string(), // Placeholder
            recent_blockhash: "11111111111111111111111111111111".to_string(), // Placeholder
            accounts: vec![], // Placeholder
            log_messages,
            block_time,
        });
    }
    None
}

/// Parse a block response from a JSON-RPC call
fn parse_block_response(slot: u64, json: &serde_json::Value) -> Option<BlockData> {
    if let Some(result) = json.get("result") {
        let blockhash = result.get("blockhash")?.as_str()?.to_string();
        let parent_slot = result.get("parentSlot")?.as_u64()?;
        let parent_blockhash = result.get("previousBlockhash")?.as_str()?.to_string();
        let block_time = result.get("blockTime").and_then(|bt| bt.as_i64());
        let block_height = result.get("blockHeight").and_then(|bh| bh.as_u64());
        let transaction_count = Some(result.get("transactions")?.as_array()?.len() as u64);
        
        return Some(BlockData {
            slot,
            blockhash,
            parent_slot,
            parent_blockhash,
            block_time,
            block_height,
            transaction_count,
            status: Some(1), // Confirmed
            leader: None,
        });
    }
    None
} 
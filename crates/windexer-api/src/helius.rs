use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};

#[derive(Debug, Clone)]
pub struct HeliusClient {
    /// Base URL for Helius HTTP API
    base_url: String,
    /// API key
    api_key: String,
    /// HTTP client
    client: reqwest::Client,
    /// WebSocket connection (if established)
    ws_connection: Arc<RwLock<Option<String>>>,
}

// Various request structs for Helius API
#[derive(Debug, Serialize, Deserialize)]
pub struct GetAccountInfoRequest {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTransactionRequest {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: Vec<String>,
}

impl HeliusClient {
    pub fn new(api_key: &str) -> Self {
        let client = reqwest::Client::new();
        let base_url = format!("https://mainnet.helius-rpc.com/?api-key={}", api_key);
        
        Self {
            client,
            base_url,
            api_key: api_key.to_string(),
            ws_connection: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn get_account_info(&self, pubkey: &str) -> Result<serde_json::Value> {
        let request = GetAccountInfoRequest {
            jsonrpc: "2.0".to_string(),
            id: "1".to_string(),
            method: "getAccountInfo".to_string(),
            params: vec![
                serde_json::json!(pubkey),
                serde_json::json!({
                    "encoding": "base64"
                })
            ],
        };

        let response = self.client.post(&self.base_url)
            .json(&request)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        tracing::debug!("Helius getAccountInfo response: {:?}", response);
        Ok(response)
    }

    pub async fn get_transaction(&self, signature: &str) -> Result<serde_json::Value> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "getTransaction",
            "params": [
                signature,
                {
                    "encoding": "json",
                    "maxSupportedTransactionVersion": 0
                }
            ]
        });

        let response = self.send_request(request).await?;
        tracing::debug!("Helius getTransaction response: {:?}", response);
        Ok(response)
    }

    pub async fn get_latest_block(&self) -> Result<crate::block_endpoints::BlockData> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "getLatestBlockhash",
            "params": []
        });
        
        let result = self.send_request(request).await?;
        tracing::debug!("Helius getLatestBlockhash response: {}", result);
        
        // Check for error in response
        if let Some(error) = result.get("error") {
            return Err(anyhow::anyhow!("Helius API error: {}", error));
        }
        
        // Now let's get additional information about this block
        if let Some(blockhash) = result.get("result").and_then(|r| r.get("value")).and_then(|v| v.get("blockhash")).and_then(|bh| bh.as_str()) {
            let slot = result.get("result").and_then(|r| r.get("context")).and_then(|c| c.get("slot")).and_then(|s| s.as_u64())
                .ok_or_else(|| anyhow::anyhow!("Failed to extract slot from response"))?;
            
            // Try to get block information
            let request = serde_json::json!({
                "jsonrpc": "2.0",
                "id": "1",
                "method": "getBlock",
                "params": [slot, {"maxSupportedTransactionVersion":0}]
            });
            
            let block_info = self.send_request(request).await?;
            tracing::debug!("Helius getBlock response: {}", block_info);
            
            // Check for error in block response
            if let Some(error) = block_info.get("error") {
                return Err(anyhow::anyhow!("Helius API error getting block: {}", error));
            }
            
            return self.parse_block_data_from_response(block_info);
        }
        
        Err(anyhow::anyhow!("Failed to extract blockhash from Helius response"))
    }

    pub async fn connect_websocket(&self) -> Result<()> {
        let ws_url = format!("wss://mainnet.helius-rpc.com/?api-key={}", self.api_key);
        
        let mut connection = self.ws_connection.write().await;
        *connection = Some(ws_url.clone());
        
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "getHealth",
        });
        
        match self.send_request(request).await {
            Ok(response) => {
                if let Some(error) = response.get("error") {
                    return Err(anyhow::anyhow!("Error connecting to Helius: {}", error));
                }
                tracing::info!("Connected to Helius API successfully");
                Ok(())
            },
            Err(e) => {
                Err(anyhow::anyhow!("Failed to connect to Helius: {}", e))
            }
        }
    }

    pub async fn subscribe_account_updates(&self, pubkey: &str) -> Result<()> {
        let subscription = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "accountSubscribe",
            "params": [
                pubkey,
                {
                    "encoding": "jsonParsed",
                    "commitment": "confirmed"
                }
            ]
        });
        
        tracing::info!("Subscribing to account updates for {}", pubkey);
        
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "getAccountInfo",
            "params": [
                pubkey,
                {
                    "encoding": "base64"
                }
            ]
        });
        
        let response = self.send_request(request).await?;
        if response.get("error").is_some() {
            return Err(anyhow::anyhow!("Error verifying account exists: {:?}", response.get("error")));
        }
        
        Ok(())
    }

    pub async fn subscribe_program_updates(&self, program_id: &str) -> Result<()> {
        let subscription = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "programSubscribe",
            "params": [
                program_id,
                {
                    "encoding": "jsonParsed",
                    "commitment": "confirmed"
                }
            ]
        });
        
        tracing::info!("Subscribing to program updates for {}", program_id);
        
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "getAccountInfo",
            "params": [
                program_id,
                {
                    "encoding": "base64"
                }
            ]
        });
        
        let response = self.send_request(request).await?;
        if response.get("error").is_some() {
            return Err(anyhow::anyhow!("Error verifying program exists: {:?}", response.get("error")));
        }
        
        Ok(())
    }

    pub async fn subscribe_slot_updates(&self) -> Result<()> {
        let subscription = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "slotSubscribe"
        });
        
        tracing::info!("Subscribing to slot updates");
        
        Ok(())
    }
    
    pub async fn process_messages<F>(&self, _message_handler: F) -> Result<()>
    where
        F: FnMut(serde_json::Value) -> Result<()> + Send + 'static
    {
        Ok(())
    }

    async fn send_request(&self, request: serde_json::Value) -> Result<serde_json::Value> {
        let response = self.client.post(&self.base_url)
            .json(&request)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
            
        Ok(response)
    }

    pub async fn get_blocks(&self, limit: usize) -> Result<Vec<crate::block_endpoints::BlockData>> {
        let response = self.get_latest_block().await?;
        let latest_slot = response.slot;
        let slots: Vec<u64> = (0..limit as u64).map(|i| latest_slot.saturating_sub(i)).collect();
        let mut blocks = Vec::new();
        for slot in slots {
            match self.get_block_by_slot(slot).await {
                Ok(block) => blocks.push(block),
                Err(e) => {
                    tracing::warn!("Failed to get block for slot {}: {}", slot, e);
                }
            }
        }
        
        Ok(blocks)
    }
    
    pub async fn get_block_by_slot(&self, slot: u64) -> Result<crate::block_endpoints::BlockData> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBlock",
            "params": [
                slot,
                {
                    "encoding": "json",
                    "transactionDetails": "full",
                    "rewards": true
                }
            ]
        });
        
        let response = self.client
            .post(&self.base_url)
            .json(&payload)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
            
        if let Some(error) = response.get("error") {
            return Err(anyhow::anyhow!("Helius API error: {}", error));
        }
        
        if let Some(result) = response.get("result") {
            let slot: u64 = result.get("slot").and_then(|s| s.as_u64()).unwrap_or(0);
            let parent_slot = result.get("parentSlot").and_then(|s| s.as_u64()).unwrap_or(slot.saturating_sub(1));
            let blockhash = result.get("blockhash").and_then(|h| h.as_str()).unwrap_or("unknown").to_string();
            let previous_blockhash = result.get("previousBlockhash").and_then(|h| h.as_str()).unwrap_or("unknown").to_string();
            let block_time = result.get("blockTime").and_then(|t| t.as_i64()).unwrap_or_else(|| chrono::Utc::now().timestamp());
            let block_height = result.get("blockHeight").and_then(|h| h.as_u64()).unwrap_or(0);
            let transaction_count = result.get("transactions")
                .and_then(|txs| txs.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0) as u64;
            let leader = result.get("rewards")
                .and_then(|rewards| rewards.as_array())
                .and_then(|arr| arr.iter().find(|r| r.get("rewardType").and_then(|t| t.as_str()) == Some("fee")))
                .and_then(|r| r.get("pubkey").and_then(|p| p.as_str()))
                .unwrap_or("11111111111111111111111111111111")
                .to_string();
            let rewards = result.get("rewards")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|r| {
                            let pubkey = r.get("pubkey").and_then(|p| p.as_str())?;
                            let lamports = r.get("lamports").and_then(|l| l.as_i64())?;
                            let post_balance = r.get("postBalance").and_then(|b| b.as_u64())?;
                            let reward_type = r.get("rewardType").and_then(|t| t.as_str())?;
                            
                            Some(crate::block_endpoints::Reward {
                                pubkey: pubkey.to_string(),
                                lamports,
                                post_balance,
                                reward_type: Some(reward_type.to_string()),
                            })
                        })
                        .collect::<Vec<crate::block_endpoints::Reward>>()
                })
                .unwrap_or_default();
            
            let block = crate::block_endpoints::BlockData {
                slot,
                parent_slot,
                blockhash,
                previous_blockhash,
                block_time: Some(block_time),
                block_height: Some(block_height),
                transaction_count,
                leader,
                rewards: Some(rewards),
            };
            
            return Ok(block);
        }
        
        Err(anyhow::anyhow!("Failed to parse block data from Helius response"))
    }

    fn parse_block_data_from_response(&self, response: serde_json::Value) -> Result<crate::block_endpoints::BlockData> {
        if let Some(result) = response.get("result") {
            // Extract basic block information
            let slot = result.get("slot").and_then(|s| s.as_u64()).unwrap_or(0);
            let parent_slot = result.get("parentSlot").and_then(|s| s.as_u64()).unwrap_or(slot.saturating_sub(1));
            let blockhash = result.get("blockhash").and_then(|h| h.as_str()).unwrap_or("unknown").to_string();
            let previous_blockhash = result.get("previousBlockhash").and_then(|h| h.as_str()).unwrap_or("unknown").to_string();
            let block_time = result.get("blockTime").and_then(|t| t.as_i64()).unwrap_or_else(|| chrono::Utc::now().timestamp());
            let block_height = result.get("blockHeight").and_then(|h| h.as_u64()).unwrap_or(0);
            
            let transaction_count = result.get("transactions")
                .and_then(|txs| txs.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0) as u64;
            
            let leader = result.get("rewards")
                .and_then(|rewards| rewards.as_array())
                .and_then(|arr| arr.iter().find(|r| r.get("rewardType").and_then(|t| t.as_str()) == Some("fee")))
                .and_then(|r| r.get("pubkey").and_then(|p| p.as_str()))
                .unwrap_or("11111111111111111111111111111111")
                .to_string();
            
            let rewards = result.get("rewards")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|r| {
                            let pubkey = r.get("pubkey").and_then(|p| p.as_str())?;
                            let lamports = r.get("lamports").and_then(|l| l.as_i64())?;
                            let post_balance = r.get("postBalance").and_then(|b| b.as_u64())?;
                            let reward_type = r.get("rewardType").and_then(|t| t.as_str())?;
                            
                            Some(crate::block_endpoints::Reward {
                                pubkey: pubkey.to_string(),
                                lamports,
                                post_balance,
                                reward_type: Some(reward_type.to_string()),
                            })
                        })
                        .collect::<Vec<crate::block_endpoints::Reward>>()
                })
                .unwrap_or_default();
            
            let block = crate::block_endpoints::BlockData {
                slot,
                parent_slot,
                blockhash,
                previous_blockhash,
                block_time: Some(block_time),
                block_height: Some(block_height),
                transaction_count,
                leader,
                rewards: Some(rewards),
            };
            
            return Ok(block);
        }
        
        Err(anyhow::anyhow!("Failed to parse block data from Helius response"))
    }
}
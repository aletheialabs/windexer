use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast, mpsc};
use anyhow::Result;
use serde_json::Value;
use chrono;

use crate::transaction_endpoints::TransactionData;
use crate::helius::HeliusClient;

pub struct TransactionDataManager {
    helius_client: Arc<HeliusClient>,
    
    cache: Arc<RwLock<HashMap<String, TransactionData>>>,
    
    recent_transactions: Arc<RwLock<VecDeque<String>>>,
    
    program_transactions: Arc<RwLock<HashMap<String, VecDeque<String>>>>,
    
    account_transactions: Arc<RwLock<HashMap<String, VecDeque<String>>>>,
    
    update_sender: broadcast::Sender<TransactionData>,
    
    initialized: Arc<RwLock<bool>>,
    
    max_cache_size: usize,
    
    max_recent_transactions: usize,
}

impl TransactionDataManager {
    pub fn new(helius_client: Arc<HeliusClient>) -> Self {
        let (tx, _) = broadcast::channel(10000); // Buffer for 10,000 transaction updates
        
        Self {
            helius_client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            recent_transactions: Arc::new(RwLock::new(VecDeque::new())),
            program_transactions: Arc::new(RwLock::new(HashMap::new())),
            account_transactions: Arc::new(RwLock::new(HashMap::new())),
            update_sender: tx,
            initialized: Arc::new(RwLock::new(false)),
            max_cache_size: 100000, // Store up to 100,000 transactions in cache
            max_recent_transactions: 1000, // Keep 1,000 recent transactions per program/account
        }
    }
    
    /// Initialize the manager
    pub async fn initialize(&self) -> Result<()> {
        let mut initialized = self.initialized.write().await;
        
        if *initialized {
            return Ok(());
        }
        
        match self.helius_client.connect_websocket().await {
            Ok(_) => {
                tracing::info!("Successfully connected to Helius WebSocket");
            }
            Err(e) => {
                tracing::error!("Failed to connect to Helius WebSocket: {}", e);
                return Err(anyhow::anyhow!("Failed to connect to Helius WebSocket: {}", e));
            }
        }
        
        let initial_programs = vec![
            "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB", // Jupiter Aggregator
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", // Token Program
            "11111111111111111111111111111111", // System Program
        ];
        
        for program in initial_programs {
            if let Err(e) = self.subscribe_to_program(program).await {
                tracing::warn!("Failed to subscribe to program {}: {}", program, e);
            }
        }
        
        *initialized = true;
        
        Ok(())
    }
    
    pub async fn subscribe_to_program(&self, program_id: &str) -> Result<()> {
        // Subscribe via Helius
        tracing::info!("Subscribing to program: {}", program_id);
        
        // Add to our tracking
        let mut program_txs = self.program_transactions.write().await;
        program_txs.entry(program_id.to_string()).or_insert_with(VecDeque::new);
        
        // Subscribe using the Helius client
        self.helius_client.subscribe_program_updates(program_id).await
    }
    
    pub async fn get_transaction(&self, signature: &str) -> Result<TransactionData> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(tx) = cache.get(signature) {
                return Ok(tx.clone());
            }
        }
        
        let response = self.helius_client.get_transaction(signature).await?;
        
        tracing::debug!("Helius transaction response: {:?}", response);
        
        if let Some(error) = response.get("error") {
            return Err(anyhow::anyhow!("Helius API error: {}", error));
        }
        
        let result = response.get("result").ok_or_else(|| anyhow::anyhow!("Missing result field in response"))?;
        
        if result.is_null() {
            return Err(anyhow::anyhow!("Transaction not found"));
        }
        
        let slot = result.get("slot").and_then(|s| s.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Missing slot field in result"))?;
        
        let block_time = result.get("blockTime").and_then(|b| b.as_i64());
        
        let meta = result.get("meta").ok_or_else(|| anyhow::anyhow!("Missing meta field in result"))?;
        let err = meta.get("err").and_then(|e| {
            if e.is_null() {
                None
            } else {
                Some(e.clone())
            }
        });
        let fee = meta.get("fee").and_then(|f| f.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Missing fee field in meta"))?;
        
        let logs = meta.get("logMessages").and_then(|l| {
            if l.is_array() {
                Some(l.as_array().unwrap()
                    .iter()
                    .map(|entry| entry.as_str().unwrap_or("").to_string())
                    .collect())
            } else {
                None
            }
        });
        
        let transaction = result.get("transaction").ok_or_else(|| anyhow::anyhow!("Missing transaction field in result"))?;
        let message = transaction.get("message").ok_or_else(|| anyhow::anyhow!("Missing message field in transaction"))?;
        
        let recent_blockhash = message.get("recentBlockhash")
            .and_then(|b| b.as_str())
            .unwrap_or("")
            .to_string();
        
        let account_keys = message.get("accountKeys")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|key| key.as_str().unwrap_or("").to_string())
                    .collect()
            })
            .unwrap_or_else(Vec::new);
        
        let program_ids = message.get("instructions")
            .and_then(|i| i.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|inst| {
                        inst.get("programId").and_then(|p| p.as_str()).map(|s| s.to_string())
                    })
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(|| {
                account_keys.clone()
            });
        
        let instructions = message.get("instructions")
            .and_then(|i| i.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|inst| {
                        let program_id = inst.get("programId").and_then(|p| p.as_str())?;
                        let accounts = inst.get("accounts")
                            .and_then(|a| a.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|idx| {
                                        idx.as_u64().and_then(|i| account_keys.get(i as usize)).cloned()
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        
                        let data = inst.get("data").and_then(|d| d.as_str()).unwrap_or("").to_string();
                        
                        Some(crate::transaction_endpoints::InstructionData {
                            program_id: program_id.to_string(),
                            accounts,
                            data,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        
        let tx = TransactionData {
            signature: signature.to_string(),
            slot,
            block_time,
            err: err.clone(),
            fee,
            recent_blockhash,
            program_ids,
            accounts: account_keys,
            logs,
            instructions,
            success: err.is_none(),
        };
        
        {
            let mut cache = self.cache.write().await;
            cache.insert(signature.to_string(), tx.clone());
            
            let mut recent = self.recent_transactions.write().await;
            recent.push_back(signature.to_string());
            
            if recent.len() > self.max_recent_transactions {
                recent.pop_front();
            }
            
            for program_id in &tx.program_ids {
                let mut program_txs = self.program_transactions.write().await;
                let queue = program_txs.entry(program_id.clone()).or_insert_with(VecDeque::new);
                queue.push_back(signature.to_string());
                
                // Limit the queue size
                if queue.len() > self.max_recent_transactions {
                    queue.pop_front();
                }
            }
            
            for account in &tx.accounts {
                let mut account_txs = self.account_transactions.write().await;
                let queue = account_txs.entry(account.clone()).or_insert_with(VecDeque::new);
                queue.push_back(signature.to_string());
                
                if queue.len() > self.max_recent_transactions {
                    queue.pop_front();
                }
            }
        }
        
        Ok(tx)
    }
    
    pub async fn get_recent_transactions(&self, limit: usize) -> Result<Vec<TransactionData>> {
        let mut txs = Vec::new();
        
        let signatures = {
            let recent = self.recent_transactions.read().await;
            recent.iter().rev().take(limit).cloned().collect::<Vec<_>>()
        };
        
        for signature in signatures {
            if let Ok(tx) = self.get_transaction(&signature).await {
                txs.push(tx);
            }
        }
        
        Ok(txs)
    }
    
    pub async fn get_transactions_by_program(&self, program_id: &str, limit: usize) -> Result<Vec<TransactionData>> {
        let mut txs = Vec::new();
        
        let signatures = {
            let program_txs = self.program_transactions.read().await;
            if let Some(program_queue) = program_txs.get(program_id) {
                program_queue.iter().rev().take(limit).cloned().collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        };
        
        for signature in signatures {
            if let Ok(tx) = self.get_transaction(&signature).await {
                txs.push(tx);
            }
        }
        
        Ok(txs)
    }
    
    pub async fn get_transactions_by_account(&self, account: &str, limit: usize) -> Result<Vec<TransactionData>> {
        let mut txs = Vec::new();
        
        let signatures = {
            let account_txs = self.account_transactions.read().await;
            if let Some(account_queue) = account_txs.get(account) {
                account_queue.iter().rev().take(limit).cloned().collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        };
        
        for signature in signatures {
            if let Ok(tx) = self.get_transaction(&signature).await {
                txs.push(tx);
            }
        }
        
        Ok(txs)
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<TransactionData> {
        self.update_sender.subscribe()
    }
}
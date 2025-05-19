use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast, mpsc};
use anyhow::Result;
use serde_json::Value;

use crate::transaction_endpoints::TransactionData;
use crate::helius::HeliusClient;

/// Manager for transaction data and WebSocket connections
pub struct TransactionDataManager {
    /// Helius client for fetching and subscribing to transaction data
    helius_client: Arc<HeliusClient>,
    
    /// Transaction data cache (limited to recent transactions)
    cache: Arc<RwLock<HashMap<String, TransactionData>>>,
    
    /// Recent transaction queue
    recent_transactions: Arc<RwLock<VecDeque<String>>>,
    
    /// Recent transactions by program
    program_transactions: Arc<RwLock<HashMap<String, VecDeque<String>>>>,
    
    /// Recent transactions by account
    account_transactions: Arc<RwLock<HashMap<String, VecDeque<String>>>>,
    
    /// Broadcast channel for transaction updates
    update_sender: broadcast::Sender<TransactionData>,
    
    /// Is the manager initialized?
    initialized: Arc<RwLock<bool>>,
    
    /// Max cache size
    max_cache_size: usize,
    
    /// Max recent transactions
    max_recent_transactions: usize,
}

impl TransactionDataManager {
    /// Create a new transaction data manager
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
        
        // Connect to WebSocket
        self.helius_client.connect_websocket().await?;
        
        // We could subscribe to transactions, but that would be very noisy
        // Instead, we'll focus on specific programs that we're interested in
        
        let initial_programs = vec![
            "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB", // Jupiter Aggregator
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", // Token Program
            "11111111111111111111111111111111", // System Program
        ];
        
        for program in initial_programs {
            self.subscribe_to_program(program).await?;
        }
        
        // Set up WebSocket message handling
        let cache = self.cache.clone();
        let recent_transactions = self.recent_transactions.clone();
        let program_transactions = self.program_transactions.clone();
        let account_transactions = self.account_transactions.clone();
        let update_sender = self.update_sender.clone();
        let max_cache_size = self.max_cache_size;
        let max_recent_transactions = self.max_recent_transactions;
        
        // Create a closure that implements FnMut to match the required signature
        let mut message_handler = move |message: serde_json::Value| -> Result<()> {
            let cache = cache.clone();
            let recent_transactions = recent_transactions.clone();
            let program_transactions = program_transactions.clone();
            let account_transactions = account_transactions.clone();
            let update_sender = update_sender.clone();
            
            // Process transaction update message
            // In a real implementation, this would parse the Solana transaction data
            // and update our caches
            
            if let Some(method) = message.get("method") {
                if let Some(method_str) = method.as_str() {
                    if method_str == "signatureNotification" {
                        if let Some(params) = message.get("params") {
                            if let Some(params_array) = params.as_array() {
                                if params_array.len() >= 2 {
                                    if let Some(result) = params_array[1].get("result") {
                                        if let Some(context) = result.get("context") {
                                            if let Some(value) = result.get("value") {
                                                // Extract transaction data
                                                let signature = if let Some(sig) = params_array[0].as_str() {
                                                    sig.to_string()
                                                } else {
                                                    return Ok(());
                                                };
                                                
                                                let slot = context.get("slot")
                                                    .and_then(Value::as_u64)
                                                    .unwrap_or(0);
                                                
                                                // For a real implementation, we would extract all transaction data
                                                // This is a simplified version
                                                let tx = TransactionData {
                                                    signature: signature.clone(),
                                                    slot,
                                                    block_time: Some(chrono::Utc::now().timestamp()),
                                                    err: None,
                                                    fee: 5000, // Would extract from transaction
                                                    recent_blockhash: "blockhash".to_string(), // Would extract from transaction
                                                    program_ids: vec!["program".to_string()], // Would extract from transaction
                                                    accounts: vec!["account".to_string()], // Would extract from transaction
                                                    logs: Some(vec!["log".to_string()]), // Would extract from transaction
                                                };
                                                
                                                // Spawn a task to update caches asynchronously
                                                // We can't use await directly in this FnMut closure
                                                tokio::spawn(async move {
                                                    // Update cache
                                                    {
                                                        let mut cache_guard = cache.write().await;
                                                        
                                                        // If cache is full, remove oldest transaction
                                                        if cache_guard.len() >= max_cache_size {
                                                            let mut recent_guard = recent_transactions.write().await;
                                                            if let Some(oldest) = recent_guard.pop_front() {
                                                                cache_guard.remove(&oldest);
                                                            }
                                                        }
                                                        
                                                        cache_guard.insert(signature.clone(), tx.clone());
                                                    }
                                                    
                                                    // Update recent transactions
                                                    {
                                                        let mut recent_guard = recent_transactions.write().await;
                                                        recent_guard.push_back(signature.clone());
                                                        
                                                        // Limit size
                                                        while recent_guard.len() > max_recent_transactions {
                                                            recent_guard.pop_front();
                                                        }
                                                    }
                                                    
                                                    // Update program transactions
                                                    for program_id in &tx.program_ids {
                                                        let mut program_guard = program_transactions.write().await;
                                                        let program_txs = program_guard.entry(program_id.clone())
                                                            .or_insert_with(VecDeque::new);
                                                        
                                                        program_txs.push_back(signature.clone());
                                                        
                                                        // Limit size
                                                        while program_txs.len() > max_recent_transactions {
                                                            program_txs.pop_front();
                                                        }
                                                    }
                                                    
                                                    // Update account transactions
                                                    for account in &tx.accounts {
                                                        let mut account_guard = account_transactions.write().await;
                                                        let account_txs = account_guard.entry(account.clone())
                                                            .or_insert_with(VecDeque::new);
                                                        
                                                        account_txs.push_back(signature.clone());
                                                        
                                                        // Limit size
                                                        while account_txs.len() > max_recent_transactions {
                                                            account_txs.pop_front();
                                                        }
                                                    }
                                                    
                                                    // Broadcast update
                                                    let _ = update_sender.send(tx);
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            Ok(())
        };
        
        self.helius_client.process_messages(message_handler).await?;
        
        *initialized = true;
        
        Ok(())
    }
    
    /// Subscribe to a Solana program for transaction updates
    pub async fn subscribe_to_program(&self, program_id: &str) -> Result<()> {
        // Subscribe via Helius
        // In a real implementation, this would use a specialized subscription method
        // For now, we'll just simulate it
        tracing::info!("Subscribing to program: {}", program_id);
        
        // Add to our tracking
        let mut program_txs = self.program_transactions.write().await;
        program_txs.entry(program_id.to_string()).or_insert_with(VecDeque::new);
        
        Ok(())
    }
    
    /// Get transaction by signature
    pub async fn get_transaction(&self, signature: &str) -> Result<TransactionData> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(tx) = cache.get(signature) {
                return Ok(tx.clone());
            }
        }
        
        // Not in cache, fetch from Helius
        let tx_info = self.helius_client.get_transaction(signature).await?;
        
        // Parse transaction data
        // In a real implementation, this would parse the Solana transaction data
        // This is a simplified version
        if let Some(result) = tx_info.get("result") {
            let slot = result.get("slot")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            
            let block_time = result.get("blockTime")
                .and_then(Value::as_i64);
            
            let meta = result.get("meta").unwrap_or(&Value::Null);
            
            let fee = meta.get("fee")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            
            let err = meta.get("err").and_then(|v| {
                if v.is_null() { None } else { Some(v.clone()) }
            });
            
            let transaction = result.get("transaction").unwrap_or(&Value::Null);
            
            let message = transaction.get("message").unwrap_or(&Value::Null);
            
            let recent_blockhash = message.get("recentBlockhash")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            
            // Extract program IDs and accounts
            let program_ids = Vec::new(); // Would extract from transaction
            let accounts = Vec::new(); // Would extract from transaction
            
            // Extract logs
            let logs = meta.get("logMessages")
                .and_then(Value::as_array)
                .map(|logs| {
                    logs.iter()
                        .filter_map(|log| log.as_str())
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                });
            
            let tx = TransactionData {
                signature: signature.to_string(),
                slot,
                block_time,
                err,
                fee,
                recent_blockhash,
                program_ids,
                accounts,
                logs,
            };
            
            // Update cache
            let mut cache = self.cache.write().await;
            cache.insert(signature.to_string(), tx.clone());
            
            return Ok(tx);
        }
        
        // If we get here, something went wrong with parsing
        Err(anyhow::anyhow!("Failed to parse transaction data"))
    }
    
    /// Get recent transactions
    pub async fn get_recent_transactions(&self, limit: usize) -> Result<Vec<TransactionData>> {
        let mut txs = Vec::new();
        
        // Get recent transaction signatures
        let signatures = {
            let recent = self.recent_transactions.read().await;
            recent.iter().rev().take(limit).cloned().collect::<Vec<_>>()
        };
        
        // Get transaction data for each signature
        for signature in signatures {
            if let Ok(tx) = self.get_transaction(&signature).await {
                txs.push(tx);
            }
        }
        
        Ok(txs)
    }
    
    /// Get transactions by program ID
    pub async fn get_transactions_by_program(&self, program_id: &str, limit: usize) -> Result<Vec<TransactionData>> {
        let mut txs = Vec::new();
        
        // Get recent transaction signatures for the program
        let signatures = {
            let program_txs = self.program_transactions.read().await;
            if let Some(program_queue) = program_txs.get(program_id) {
                program_queue.iter().rev().take(limit).cloned().collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        };
        
        // Get transaction data for each signature
        for signature in signatures {
            if let Ok(tx) = self.get_transaction(&signature).await {
                txs.push(tx);
            }
        }
        
        Ok(txs)
    }
    
    /// Get transactions by account
    pub async fn get_transactions_by_account(&self, account: &str, limit: usize) -> Result<Vec<TransactionData>> {
        let mut txs = Vec::new();
        
        // Get recent transaction signatures for the account
        let signatures = {
            let account_txs = self.account_transactions.read().await;
            if let Some(account_queue) = account_txs.get(account) {
                account_queue.iter().rev().take(limit).cloned().collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        };
        
        // Get transaction data for each signature
        for signature in signatures {
            if let Ok(tx) = self.get_transaction(&signature).await {
                txs.push(tx);
            }
        }
        
        Ok(txs)
    }
    
    /// Get a subscription to transaction updates
    pub fn subscribe(&self) -> broadcast::Receiver<TransactionData> {
        self.update_sender.subscribe()
    }
}
// crates/windexer-api/src/solana_client.rs

use anyhow::Result;
use serde::{Serialize, Deserialize};
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

use crate::endpoints::{TransactionData, AccountData};

/// Supported Solana RPC clusters
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SolanaCluster {
    /// Mainnet Beta
    MainnetBeta,
    /// Testnet
    Testnet,
    /// Devnet
    Devnet,
    /// Localnet
    Localnet,
    /// Custom RPC
    Custom,
}

impl SolanaCluster {
    /// Get the RPC URL for the cluster
    pub fn url(&self) -> String {
        match self {
            SolanaCluster::MainnetBeta => "https://api.mainnet-beta.solana.com".to_string(),
            SolanaCluster::Testnet => "https://api.testnet.solana.com".to_string(),
            SolanaCluster::Devnet => "https://api.devnet.solana.com".to_string(),
            SolanaCluster::Localnet => "http://localhost:8899".to_string(),
            SolanaCluster::Custom => std::env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "http://localhost:8899".to_string()),
        }
    }
}

/// Solana RPC client
#[derive(Debug, Clone)]
pub struct SolanaClient {
    /// HTTP client
    client: Client,
    /// RPC URL
    rpc_url: String,
    /// Current cluster
    cluster: SolanaCluster,
}

/// Solana RPC response structure
#[derive(Debug, Serialize, Deserialize)]
struct RpcResponse<T> {
    jsonrpc: String,
    id: u64,
    result: T,
}

/// Solana RPC error structure
#[derive(Debug, Serialize, Deserialize)]
struct RpcError {
    jsonrpc: String,
    id: u64,
    error: RpcErrorDetail,
}

/// Solana RPC error detail
#[derive(Debug, Serialize, Deserialize)]
struct RpcErrorDetail {
    code: i64,
    message: String,
}

/// Solana transaction response
#[derive(Debug, Serialize, Deserialize)]
struct TransactionResponse {
    slot: u64,
    transaction: TransactionInfo,
    blockTime: Option<i64>,
    meta: Option<TransactionMeta>,
}

/// Solana transaction info
#[derive(Debug, Serialize, Deserialize)]
struct TransactionInfo {
    signatures: Vec<String>,
    message: MessageInfo,
}

/// Solana message info
#[derive(Debug, Serialize, Deserialize)]
struct MessageInfo {
    accountKeys: Vec<String>,
    header: MessageHeader,
    recentBlockhash: String,
    instructions: Vec<InstructionInfo>,
}

/// Solana message header
#[derive(Debug, Serialize, Deserialize)]
struct MessageHeader {
    numRequiredSignatures: u8,
    numReadonlySignedAccounts: u8,
    numReadonlyUnsignedAccounts: u8,
}

/// Solana instruction info
#[derive(Debug, Serialize, Deserialize)]
struct InstructionInfo {
    programIdIndex: u8,
    accounts: Vec<u8>,
    data: String,
}

/// Solana transaction meta
#[derive(Debug, Serialize, Deserialize)]
struct TransactionMeta {
    err: Option<serde_json::Value>,
    fee: u64,
    preBalances: Vec<u64>,
    postBalances: Vec<u64>,
}

/// Solana account info
#[derive(Debug, Serialize, Deserialize)]
struct AccountInfo {
    lamports: u64,
    owner: String,
    data: Vec<String>,
    executable: bool,
    rentEpoch: u64,
}

impl SolanaClient {
    /// Create a new Solana client
    pub fn new(cluster: SolanaCluster) -> Self {
        let client = Client::new();
        let rpc_url = cluster.url();
        
        info!("Initializing Solana client for {}", rpc_url);
        
        Self {
            client,
            rpc_url,
            cluster,
        }
    }
    
    /// Get recent transactions
    pub async fn get_recent_transactions(&self, limit: usize) -> Result<Vec<TransactionData>> {
        info!("Fetching recent transactions from {}", self.rpc_url);
        
        // Get recent blockhash for recent blocks
        let blockhash_response = self.client
            .post(&self.rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getRecentBlockhash",
                "params": []
            }))
            .send()
            .await?;
            
        let blockhash_data: RpcResponse<serde_json::Value> = blockhash_response.json().await?;
        let blockhash = blockhash_data.result.get("blockhash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Failed to get recent blockhash"))?;
            
        // Get recent block
        let block_response = self.client
            .post(&self.rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getConfirmedSignaturesForAddress2",
                "params": [
                    "11111111111111111111111111111111", // System program
                    {
                        "limit": limit
                    }
                ]
            }))
            .send()
            .await?;
        
        let block_data: RpcResponse<Vec<serde_json::Value>> = block_response.json().await?;
        let signatures: Vec<String> = block_data.result
            .iter()
            .map(|sig| sig.get("signature").unwrap().as_str().unwrap().to_string())
            .collect();
        
        let mut transactions = Vec::new();
        
        for signature in signatures {
            let tx_response = self.client
                .post(&self.rpc_url)
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "getConfirmedTransaction",
                    "params": [
                        signature,
                        "json"
                    ]
                }))
                .send()
                .await?;
                
            let tx_data: Result<RpcResponse<Option<TransactionResponse>>, reqwest::Error> = tx_response.json().await;
            
            match tx_data {
                Ok(resp) => {
                    if let Some(tx) = resp.result {
                        let accounts = tx.transaction.message.accountKeys;
                        let success = tx.meta.as_ref().map(|m| m.err.is_none()).unwrap_or(false);
                        let fee = tx.meta.as_ref().map(|m| m.fee).unwrap_or(0);
                        
                        transactions.push(TransactionData {
                            signature,
                            block_time: tx.blockTime,
                            slot: tx.slot,
                            success,
                            fee,
                            accounts,
                        });
                    }
                }
                Err(e) => {
                    warn!("Failed to parse transaction {}: {}", signature, e);
                }
            }
        }
        
        Ok(transactions)
    }
    
    /// Get accounts by addresses
    pub async fn get_accounts(&self, addresses: &[String], limit: usize) -> Result<Vec<AccountData>> {
        info!("Fetching {} accounts from {}", addresses.len(), self.rpc_url);
        
        let mut accounts = Vec::new();
        
        // If no addresses provided, get some recent accounts from transactions
        let addresses_to_fetch = if addresses.is_empty() {
            let txs = self.get_recent_transactions(limit / 2).await?;
            txs.into_iter()
                .flat_map(|tx| tx.accounts)
                .collect::<Vec<String>>()
        } else {
            addresses.to_vec()
        };
        
        // Limit accounts to fetch
        let addresses_to_fetch = addresses_to_fetch
            .into_iter()
            .take(limit)
            .collect::<Vec<String>>();
        
        for address in addresses_to_fetch {
            let account_response = self.client
                .post(&self.rpc_url)
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "getAccountInfo",
                    "params": [
                        address,
                        {
                            "encoding": "base64"
                        }
                    ]
                }))
                .send()
                .await?;
                
            let account_data: Result<RpcResponse<Option<AccountInfo>>, reqwest::Error> = account_response.json().await;
            
            match account_data {
                Ok(resp) => {
                    if let Some(account) = resp.result {
                        accounts.push(AccountData {
                            pubkey: address,
                            lamports: account.lamports,
                            owner: account.owner,
                            executable: account.executable,
                            rent_epoch: account.rentEpoch,
                            data_len: account.data.get(0).map(|d| d.len()).unwrap_or(0),
                            write_version: 0, // Not directly available from RPC
                        });
                    }
                }
                Err(e) => {
                    warn!("Failed to parse account {}: {}", address, e);
                }
            }
        }
        
        Ok(accounts)
    }
    
    /// Get validator info
    pub async fn get_validator_info(&self) -> Result<serde_json::Value> {
        let response = self.client
            .post(&self.rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getVersion",
                "params": []
            }))
            .send()
            .await?;
            
        let data: RpcResponse<serde_json::Value> = response.json().await?;
        
        // Get cluster info
        let cluster_nodes_response = self.client
            .post(&self.rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getClusterNodes",
                "params": []
            }))
            .send()
            .await?;
            
        let nodes_data: RpcResponse<Vec<serde_json::Value>> = cluster_nodes_response.json().await?;
        
        let result = serde_json::json!({
            "version": data.result,
            "cluster": match self.cluster {
                SolanaCluster::MainnetBeta => "mainnet-beta",
                SolanaCluster::Testnet => "testnet",
                SolanaCluster::Devnet => "devnet",
                SolanaCluster::Localnet => "localnet",
                SolanaCluster::Custom => "custom",
            },
            "nodes": nodes_data.result,
            "rpc_url": self.rpc_url,
        });
        
        Ok(result)
    }
} 
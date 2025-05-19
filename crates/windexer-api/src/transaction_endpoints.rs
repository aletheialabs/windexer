use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::rest::AppState;
use crate::types::{ApiResponse, ApiError};
use crate::transaction_data_manager::TransactionDataManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub err: Option<serde_json::Value>,
    pub fee: u64,
    pub recent_blockhash: String,
    pub program_ids: Vec<String>,
    pub accounts: Vec<String>,
    pub logs: Option<Vec<String>>,
    pub instructions: Vec<InstructionData>,
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct TransactionQueryParams {
    pub limit: Option<usize>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub program: Option<String>,
    pub account: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TransactionUpdateParams {
    pub program: Option<String>,
    pub account: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionData {
    pub program_id: String,
    pub accounts: Vec<String>,
    pub data: String,
}

pub async fn get_transaction(
    State(state): State<AppState>,
    Path(signature): Path<String>,
) -> Result<Json<ApiResponse<TransactionData>>, ApiError> {
    let helius_client = state.helius_client.as_ref().ok_or_else(|| {
        ApiError::Internal("Helius client not initialized".to_string())
    })?;
    
    // Try to get transaction from manager first if available
    if let Some(tx_manager) = &state.transaction_data_manager {
        match tx_manager.get_transaction(&signature).await {
            Ok(tx) => return Ok(Json(ApiResponse::success(tx))),
            Err(e) => {
                tracing::warn!("Error getting transaction from manager, falling back to direct API call: {}", e);
                // Fall through to direct API call
            }
        }
    }
    
    match helius_client.get_transaction(&signature).await {
        Ok(response) => {
            tracing::debug!("Helius transaction response: {}", response);
            
            if let Some(error) = response.get("error") {
                return Err(ApiError::NotFound(format!("Transaction not found: {}", error)));
            }
            
            if let Some(result) = response.get("result") {
                if result.is_null() {
                    return Err(ApiError::NotFound(format!("Transaction not found: {}", signature)));
                }
                
                let slot = result.get("slot").and_then(|s| s.as_u64()).ok_or_else(|| {
                    ApiError::Internal("Could not parse slot from response".to_string())
                })?;
                
                let block_time = result.get("blockTime").and_then(|b| b.as_i64());
                
                if let Some(meta) = result.get("meta") {
                    let err = meta.get("err").and_then(|e| {
                        if e.is_null() {
                            None
                        } else {
                            Some(e.clone())
                        }
                    });
                    
                    let fee = meta.get("fee").and_then(|f| f.as_u64()).unwrap_or(0);
                    
                    // Extract logs
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
                    
                    if let Some(transaction) = result.get("transaction") {
                        if let Some(message) = transaction.get("message") {
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
                                    account_keys.first().cloned().into_iter().collect()
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
                                            
                                            Some(InstructionData {
                                                program_id: program_id.to_string(),
                                                accounts,
                                                data,
                                            })
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            
                            let tx = TransactionData {
                                signature: signature.clone(),
                                slot,
                                block_time,
                                err,
                                fee,
                                recent_blockhash,
                                program_ids,
                                accounts: account_keys,
                                logs,
                                instructions,
                                success: true,
                            };
                            
                            return Ok(Json(ApiResponse::success(tx)));
                        }
                    }
                }
            }
            
            Err(ApiError::Internal("Could not parse transaction data from response".to_string()))
        }
        Err(e) => {
            tracing::error!("Error fetching transaction from Helius: {}", e);
            Err(ApiError::Internal(format!("Error fetching transaction: {}", e)))
        }
    }
}

pub async fn get_recent_transactions(
    State(state): State<AppState>,
    Query(params): Query<TransactionQueryParams>,
) -> Result<Json<ApiResponse<Vec<TransactionData>>>, ApiError> {
    let tx_manager = state.transaction_data_manager.ok_or_else(|| {
        ApiError::Internal("Transaction data manager not initialized".to_string())
    })?;
    
    // Get limit from query params
    let limit = params.limit.unwrap_or(10);
    
    // Fetch recent transactions
    match tx_manager.get_recent_transactions(limit).await {
        Ok(txs) => Ok(Json(ApiResponse::success(txs))),
        Err(e) => Err(ApiError::Internal(format!("Failed to fetch recent transactions: {}", e)))
    }
}

pub async fn get_transactions_by_program(
    State(state): State<AppState>,
    Path(program_id): Path<String>,
    Query(params): Query<TransactionQueryParams>,
) -> Result<Json<ApiResponse<Vec<TransactionData>>>, ApiError> {
    let tx_manager = state.transaction_data_manager.ok_or_else(|| {
        ApiError::Internal("Transaction data manager not initialized".to_string())
    })?;
    
    let limit = params.limit.unwrap_or(10);
    
    match tx_manager.get_transactions_by_program(&program_id, limit).await {
        Ok(txs) => Ok(Json(ApiResponse::success(txs))),
        Err(e) => Err(ApiError::Internal(format!("Failed to fetch transactions by program: {}", e)))
    }
}

pub async fn get_transactions_by_account(
    State(state): State<AppState>,
    Path(account): Path<String>,
    Query(params): Query<TransactionQueryParams>,
) -> Result<Json<ApiResponse<Vec<TransactionData>>>, ApiError> {
    let tx_manager = state.transaction_data_manager.ok_or_else(|| {
        ApiError::Internal("Transaction data manager not initialized".to_string())
    })?;
    
    let limit = params.limit.unwrap_or(10);
    
    match tx_manager.get_transactions_by_account(&account, limit).await {
        Ok(txs) => Ok(Json(ApiResponse::success(txs))),
        Err(e) => Err(ApiError::Internal(format!("Failed to fetch transactions by account: {}", e)))
    }
}

pub async fn transaction_stream(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<TransactionUpdateParams>,
) -> impl IntoResponse {
    let program = params.program;
    let account = params.account;

    ws.on_upgrade(move |socket| async move {
        handle_transaction_websocket(socket, state, program, account).await
    })
}

async fn handle_transaction_websocket(
    socket: axum::extract::ws::WebSocket,
    state: AppState,
    program: Option<String>,
    account: Option<String>,
) {
    use axum::extract::ws::Message;
    use futures::{SinkExt, StreamExt};
    use std::time::Duration;
    
    state.metrics.set_metric("active_transaction_streams", serde_json::json!(1)).await;
    
    let (sender, receiver) = socket.split();
    
    let (tx, rx) = broadcast::channel::<TransactionData>(1000);
    
    let tx_clone = tx.clone();
    let program_clone = program.clone();
    let account_clone = account.clone();
    
    let mut simulation_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        
        loop {
            interval.tick().await;
            
            let signature = format!("signature{}", fastrand::u64(..1000000));
            
            let program_ids = if let Some(ref p) = program_clone {
                vec![p.clone()]
            } else {
                vec![format!("program{}", fastrand::u64(..10))]
            };
            
            let accounts = if let Some(ref a) = account_clone {
                vec![a.clone()]
            } else {
                vec![format!("account{}", fastrand::u64(..10))]
            };
            
            let transaction = TransactionData {
                signature,
                slot: fastrand::u64(..1000000),
                block_time: Some(chrono::Utc::now().timestamp()),
                err: None,
                fee: fastrand::u64(..10000),
                recent_blockhash: format!("blockhash{}", fastrand::u64(..1000)),
                program_ids,
                accounts,
                logs: Some(vec!["Program log: Simulated transaction".to_string()]),
                instructions: Vec::new(),
                success: true,
            };
            
            let _ = tx_clone.send(transaction);
        }
    });
    
    tokio::spawn(async move {
        let mut sender = sender;
        let mut receiver = receiver;
        let mut rx = rx;
        
        loop {
            tokio::select! {
                result = receiver.next() => {
                    match result {
                        Some(Ok(Message::Text(text))) => {
                            if text == "ping" {
                                if sender.send(Message::Text("pong".to_string())).await.is_err() {
                                    break;
                                }
                            }
                        },
                        Some(Ok(Message::Close(_))) | None => break,
                        _ => {}
                    }
                },
                
                result = rx.recv() => {
                    if let Ok(transaction) = result {
                        let matches_program = program.is_none() || 
                            transaction.program_ids.iter().any(|p| Some(p) == program.as_ref());
                            
                        let matches_account = account.is_none() || 
                            transaction.accounts.iter().any(|a| Some(a) == account.as_ref());
                        
                        if matches_program && matches_account {
                            if let Ok(json) = serde_json::to_string(&transaction) {
                                if sender.send(Message::Text(json)).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        simulation_task.abort();
        
        state.metrics.set_metric("active_transaction_streams", serde_json::json!(0)).await;
    });
}

pub fn create_transaction_router() -> Router<AppState> {
    Router::new()
        .route("/transaction/:signature", get(get_transaction))
        .route("/transactions/recent", get(get_recent_transactions))
        .route("/transactions/program/:program_id", get(get_transactions_by_program))
        .route("/transactions/account/:account", get(get_transactions_by_account))
        .route("/ws/transactions", get(transaction_stream))
}

pub fn create_jito_compat_transaction_router() -> Router<AppState> {
    Router::new()
        .route("/transactions/recent", get(get_recent_transactions_jito_compat))
        .route("/transaction/:signature", get(get_transaction_by_signature_jito_compat))
        .route("/transactions/program/:pubkey", get(get_transactions_by_program_jito_compat))
        .route("/transactions/account/:pubkey", get(get_transactions_by_account_jito_compat))
}

async fn get_recent_transactions_jito_compat(
    State(state): State<AppState>,
    Query(params): Query<TransactionQueryParams>,
) -> Result<Json<Vec<TransactionData>>, ApiError> {
    let transactions = get_recent_transactions_internal(state, params).await?;
    Ok(Json(transactions))
}

async fn get_transaction_by_signature_jito_compat(
    State(state): State<AppState>,
    Path(signature): Path<String>,
) -> Result<Json<TransactionData>, ApiError> {
    let transaction = get_transaction_by_signature_internal(state, signature).await?;
    Ok(Json(transaction))
}

async fn get_transactions_by_program_jito_compat(
    State(state): State<AppState>,
    Path(pubkey): Path<String>,
    Query(params): Query<TransactionQueryParams>,
) -> Result<Json<Vec<TransactionData>>, ApiError> {
    let transactions = get_transactions_by_program_internal(state, pubkey, params).await?;
    Ok(Json(transactions))
}

async fn get_transactions_by_account_jito_compat(
    State(state): State<AppState>,
    Path(pubkey): Path<String>,
    Query(params): Query<TransactionQueryParams>,
) -> Result<Json<Vec<TransactionData>>, ApiError> {
    let transactions = get_transactions_by_account_internal(state, pubkey, params).await?;
    Ok(Json(transactions))
}

async fn get_recent_transactions_internal(
    state: AppState,
    params: TransactionQueryParams,
) -> Result<Vec<TransactionData>, ApiError> {
    let tx_manager = state.transaction_data_manager.ok_or_else(|| {
        ApiError::Internal("Transaction data manager not initialized".to_string())
    })?;
    
    let limit = params.limit.unwrap_or(10);
    
    tx_manager.get_recent_transactions(limit).await
        .map_err(|e| ApiError::Internal(format!("Failed to fetch recent transactions: {}", e)))
}

async fn get_transaction_by_signature_internal(
    state: AppState,
    signature: String,
) -> Result<TransactionData, ApiError> {
    if let Some(manager) = &state.transaction_data_manager {
        // Get transaction from manager
        match manager.get_transaction(&signature).await {
            Ok(tx) => Ok(tx),
            Err(e) => Err(ApiError::InternalError(format!("Failed to fetch transaction: {}", e)))
        }
    } else {
        // Return mock data for testing
        let slot = 100000000;
        let i = signature.chars().last().unwrap_or('0').to_digit(10).unwrap_or(0);
        Ok(create_mock_transaction(signature, slot, i as u8))
    }
}

async fn get_transactions_by_program_internal(
    state: AppState,
    pubkey: String,
    params: TransactionQueryParams,
) -> Result<Vec<TransactionData>, ApiError> {
    let limit = params.limit.unwrap_or(10).min(100);
    
    if let Some(manager) = &state.transaction_data_manager {
        let transactions = manager.get_transactions_by_program(&pubkey, limit).await
            .map_err(|e| ApiError::InternalError(format!("Failed to fetch transactions: {}", e)))?;
            
        Ok(transactions)
    } else {
        let mut transactions = Vec::new();
        for i in 0..limit {
            let slot = 100000000;
            let signature = format!("sig_{}_{}_{}", slot, pubkey.chars().take(4).collect::<String>(), i);
            let mut tx = create_mock_transaction(signature.clone(), slot, i as u8);
            
            tx.instructions.push(InstructionData {
                program_id: pubkey.clone(),
                accounts: vec!["11111111111111111111111111111111".to_string()],
                data: format!("instruction data {}", i),
            });
            
            transactions.push(tx);
        }
        Ok(transactions)
    }
}

async fn get_transactions_by_account_internal(
    state: AppState,
    pubkey: String,
    params: TransactionQueryParams,
) -> Result<Vec<TransactionData>, ApiError> {
    let limit = params.limit.unwrap_or(10).min(100);
    
    if let Some(manager) = &state.transaction_data_manager {
        let transactions = manager.get_transactions_by_account(&pubkey, limit).await
            .map_err(|e| ApiError::InternalError(format!("Failed to fetch transactions: {}", e)))?;
            
        Ok(transactions)
    } else {
        let mut transactions = Vec::new();
        for i in 0..limit {
            let slot = 100000000;
            let signature = format!("sig_{}_{}_{}", slot, pubkey.chars().take(4).collect::<String>(), i);
            let mut tx = create_mock_transaction(signature.clone(), slot, i as u8);
            
            tx.accounts.push(pubkey.clone());
            
            transactions.push(tx);
        }
        Ok(transactions)
    }
}

fn create_mock_transaction(signature: String, slot: u64, index: u8) -> TransactionData {
    // This function should not be used anymore - throw an error if called
    panic!("create_mock_transaction should not be called: mock data is disabled");
}
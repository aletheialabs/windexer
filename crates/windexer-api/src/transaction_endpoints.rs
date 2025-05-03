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

// Types for transaction data
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
}

// Query parameters for transactions
#[derive(Debug, Deserialize)]
pub struct TransactionQueryParams {
    pub limit: Option<usize>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub program: Option<String>,
    pub account: Option<String>,
}

// Query parameters for transaction updates
#[derive(Debug, Deserialize)]
pub struct TransactionUpdateParams {
    pub program: Option<String>,
    pub account: Option<String>,
}

// Get transaction by signature
pub async fn get_transaction(
    State(_state): State<AppState>,
    Path(signature): Path<String>,
) -> Result<Json<ApiResponse<TransactionData>>, ApiError> {
    // For now, return placeholder data
    let tx = TransactionData {
        signature: signature.clone(),
        slot: 100000000,
        block_time: Some(chrono::Utc::now().timestamp()),
        err: None,
        fee: 5000,
        recent_blockhash: "11111111111111111111111111111111".to_string(),
        program_ids: vec!["11111111111111111111111111111111".to_string()],
        accounts: vec!["11111111111111111111111111111111".to_string()],
        logs: Some(vec!["Program log: Hello".to_string()]),
    };

    Ok(Json(ApiResponse::success(tx)))
}

// Get recent transactions
pub async fn get_recent_transactions(
    State(_state): State<AppState>,
    Query(params): Query<TransactionQueryParams>,
) -> Result<Json<ApiResponse<Vec<TransactionData>>>, ApiError> {
    // For now, return placeholder data
    let limit = params.limit.unwrap_or(10);
    
    let transactions = (0..limit)
        .map(|i| TransactionData {
            signature: format!("signature{}", i),
            slot: 100000000 + i as u64,
            block_time: Some(chrono::Utc::now().timestamp() - i as i64),
            err: None,
            fee: 5000,
            recent_blockhash: "11111111111111111111111111111111".to_string(),
            program_ids: vec!["11111111111111111111111111111111".to_string()],
            accounts: vec!["11111111111111111111111111111111".to_string()],
            logs: Some(vec!["Program log: Hello".to_string()]),
        })
        .collect();

    Ok(Json(ApiResponse::success(transactions)))
}

// Get transactions by program ID
pub async fn get_transactions_by_program(
    State(_state): State<AppState>,
    Path(program_id): Path<String>,
    Query(params): Query<TransactionQueryParams>,
) -> Result<Json<ApiResponse<Vec<TransactionData>>>, ApiError> {
    // For now, return placeholder data
    let limit = params.limit.unwrap_or(10);
    
    let transactions = (0..limit)
        .map(|i| TransactionData {
            signature: format!("signature{}-{}", i, program_id),
            slot: 100000000 + i as u64,
            block_time: Some(chrono::Utc::now().timestamp() - i as i64),
            err: None,
            fee: 5000,
            recent_blockhash: "11111111111111111111111111111111".to_string(),
            program_ids: vec![program_id.clone()],
            accounts: vec!["11111111111111111111111111111111".to_string()],
            logs: Some(vec!["Program log: Hello".to_string()]),
        })
        .collect();

    Ok(Json(ApiResponse::success(transactions)))
}

// Get transactions by account
pub async fn get_transactions_by_account(
    State(_state): State<AppState>,
    Path(account): Path<String>,
    Query(params): Query<TransactionQueryParams>,
) -> Result<Json<ApiResponse<Vec<TransactionData>>>, ApiError> {
    // For now, return placeholder data
    let limit = params.limit.unwrap_or(10);
    
    let transactions = (0..limit)
        .map(|i| TransactionData {
            signature: format!("signature{}-{}", i, account),
            slot: 100000000 + i as u64,
            block_time: Some(chrono::Utc::now().timestamp() - i as i64),
            err: None,
            fee: 5000,
            recent_blockhash: "11111111111111111111111111111111".to_string(),
            program_ids: vec!["11111111111111111111111111111111".to_string()],
            accounts: vec![account.clone()],
            logs: Some(vec!["Program log: Hello".to_string()]),
        })
        .collect();

    Ok(Json(ApiResponse::success(transactions)))
}

// WebSocket handler for real-time transaction updates
pub async fn transaction_stream(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<TransactionUpdateParams>,
) -> impl IntoResponse {
    // Get program to filter by, if specified
    let program = params.program;
    
    // Get account to filter by, if specified
    let account = params.account;
    
    ws.on_upgrade(move |socket| async move {
        handle_transaction_websocket(socket, state, program, account).await
    })
}

// Internal function to handle the WebSocket connection
async fn handle_transaction_websocket(
    socket: axum::extract::ws::WebSocket,
    state: AppState,
    program: Option<String>,
    account: Option<String>,
) {
    use axum::extract::ws::Message;
    use futures::{SinkExt, StreamExt};
    use std::time::Duration;
    
    // Record metric for active streams
    state.metrics.set_metric("active_transaction_streams", serde_json::json!(1)).await;
    
    // Split the socket into sender and receiver
    let (sender, receiver) = socket.split();
    
    // Create a channel for transaction updates
    let (tx, rx) = broadcast::channel::<TransactionData>(1000);
    
    // Spawn a task to simulate real transaction updates
    let tx_clone = tx.clone();
    let program_clone = program.clone();
    let account_clone = account.clone();
    
    let mut simulation_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        
        loop {
            interval.tick().await;
            
            // Simulate a transaction update
            let signature = format!("signature{}", fastrand::u64(..1000000));
            
            // Generate random program IDs and accounts
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
            };
            
            let _ = tx_clone.send(transaction);
        }
    });
    
    // Use tokio::select! to handle both sending and receiving in a single task
    tokio::spawn(async move {
        let mut sender = sender;
        let mut receiver = receiver;
        let mut rx = rx;
        
        loop {
            tokio::select! {
                // Handle received messages from WebSocket
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
                
                // Handle transaction updates from broadcast channel
                result = rx.recv() => {
                    if let Ok(transaction) = result {
                        // Check if the transaction matches our filters
                        let matches_program = program.is_none() || 
                            transaction.program_ids.iter().any(|p| Some(p) == program.as_ref());
                            
                        let matches_account = account.is_none() || 
                            transaction.accounts.iter().any(|a| Some(a) == account.as_ref());
                        
                        if matches_program && matches_account {
                            // Serialize and send the transaction update
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
        
        // Cancel the simulation task when the WebSocket closes
        simulation_task.abort();
        
        // Update metric when connection ends
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
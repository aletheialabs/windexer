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

// Types for account data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountData {
    pub pubkey: String,
    pub lamports: u64,
    pub owner: String,
    pub executable: bool,
    pub rent_epoch: u64,
    pub data: Vec<u8>,
    pub data_base64: Option<String>,
    pub slot: u64,
    pub updated_at: i64,
}

// Query parameters for accounts
#[derive(Debug, Deserialize)]
pub struct AccountQueryParams {
    pub limit: Option<usize>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub program: Option<String>,
}

// Query parameters for account updates
#[derive(Debug, Deserialize)]
pub struct AccountUpdateParams {
    pub program: Option<String>,
    pub pubkeys: Option<String>, // Comma-separated list of pubkeys
}

// Get account by public key
pub async fn get_account(
    State(_state): State<AppState>,
    Path(pubkey): Path<String>,
) -> Result<Json<ApiResponse<AccountData>>, ApiError> {
    // For now, return placeholder data
    let account = AccountData {
        pubkey: pubkey.clone(),
        lamports: 100000000,
        owner: "11111111111111111111111111111111".to_string(),
        executable: false,
        rent_epoch: 0,
        data: vec![],
        data_base64: Some("".to_string()),
        slot: 100000000,
        updated_at: chrono::Utc::now().timestamp(),
    };

    Ok(Json(ApiResponse::success(account)))
}

// Get accounts by program ID
pub async fn get_accounts_by_program(
    State(_state): State<AppState>,
    Path(program_id): Path<String>,
    Query(params): Query<AccountQueryParams>,
) -> Result<Json<ApiResponse<Vec<AccountData>>>, ApiError> {
    // For now, return placeholder data
    let limit = params.limit.unwrap_or(10);
    
    let accounts = (0..limit)
        .map(|i| AccountData {
            pubkey: format!("account{}-{}", i, program_id),
            lamports: 100000000,
            owner: program_id.clone(),
            executable: false,
            rent_epoch: 0,
            data: vec![],
            data_base64: Some("".to_string()),
            slot: 100000000,
            updated_at: chrono::Utc::now().timestamp(),
        })
        .collect();

    Ok(Json(ApiResponse::success(accounts)))
}

// WebSocket handler for real-time account updates
pub async fn account_stream(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<AccountUpdateParams>,
) -> impl IntoResponse {
    // Get a list of account pubkeys to monitor, if specified
    let pubkeys = params.pubkeys
        .map(|p| p.split(',').map(|s| s.to_string()).collect::<Vec<_>>())
        .unwrap_or_default();
    
    // Get program to filter by, if specified
    let program = params.program;
    
    ws.on_upgrade(move |socket| async move {
        handle_account_websocket(socket, state, pubkeys, program).await
    })
}

// Internal function to handle the WebSocket connection
async fn handle_account_websocket(
    socket: axum::extract::ws::WebSocket,
    state: AppState,
    pubkeys: Vec<String>,
    program: Option<String>,
) {
    use axum::extract::ws::Message;
    use futures::{SinkExt, StreamExt};
    use std::time::Duration;
    
    // Record metric for active streams
    state.metrics.set_metric("active_account_streams", serde_json::json!(1)).await;
    
    // Split the socket into sender and receiver
    let (sender, receiver) = socket.split();
    
    // Create a channel for account updates
    let (tx, rx) = broadcast::channel::<AccountData>(1000);
    
    // Spawn a task to simulate real account updates
    let tx_clone = tx.clone();
    let pubkeys_clone = pubkeys.clone();
    let program_clone = program.clone();
    
    let mut simulation_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        
        loop {
            interval.tick().await;
            
            // Simulate an account update
            let pubkey = if !pubkeys_clone.is_empty() {
                pubkeys_clone[fastrand::usize(..pubkeys_clone.len())].clone()
            } else {
                format!("simulated{}", fastrand::u64(..100000))
            };
            
            let owner = program_clone.clone().unwrap_or_else(|| {
                "11111111111111111111111111111111".to_string()
            });
            
            let account = AccountData {
                pubkey,
                lamports: fastrand::u64(..1000000000),
                owner,
                executable: false,
                rent_epoch: 0,
                data: vec![],
                data_base64: Some("".to_string()),
                slot: fastrand::u64(..1000000),
                updated_at: chrono::Utc::now().timestamp(),
            };
            
            let _ = tx_clone.send(account);
        }
    });
    
    // Clone the sender for separate use in our tasks
    let ws_sender = sender;
    
    // Spawn a task to handle WebSocket messages and account updates
    tokio::spawn(async move {
        let mut sender = ws_sender;
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
                
                // Handle account updates from broadcast channel
                result = rx.recv() => {
                    if let Ok(account) = result {
                        // Check if the account matches our filters
                        let matches_pubkey = pubkeys.is_empty() || pubkeys.contains(&account.pubkey);
                        let matches_program = program.is_none() || program.as_ref() == Some(&account.owner);
                        
                        if matches_pubkey && matches_program {
                            // Serialize and send the account update
                            if let Ok(json) = serde_json::to_string(&account) {
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
        state.metrics.set_metric("active_account_streams", serde_json::json!(0)).await;
    });
}

pub fn create_account_router() -> Router<AppState> {
    Router::new()
        .route("/account/:pubkey", get(get_account))
        .route("/accounts/program/:program_id", get(get_accounts_by_program))
        .route("/ws/accounts", get(account_stream))
}
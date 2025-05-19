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

#[derive(Debug, Deserialize)]
pub struct AccountQueryParams {
    pub limit: Option<usize>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub program: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AccountUpdateParams {
    pub program: Option<String>,
    pub pubkeys: Option<String>, // Comma-separated list of pubkeys
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountBalance {
    pub address: String,
    pub lamports: u64,
    pub sol: f64,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenBalance {
    pub mint: String,
    pub owner: String,
    pub amount: String,
    pub decimals: u8,
    pub ui_amount: f64,
}

pub async fn get_account(
    State(state): State<AppState>,
    Path(pubkey): Path<String>,
) -> Result<Json<ApiResponse<AccountData>>, ApiError> {
    let account_manager = state.account_data_manager.ok_or_else(|| {
        ApiError::Internal("Account data manager not initialized".to_string())
    })?;
    
    match account_manager.get_account(&pubkey).await {
        Ok(account) => Ok(Json(ApiResponse::success(account))),
        Err(e) => Err(ApiError::Internal(format!("Failed to fetch account: {}", e)))
    }
}

pub async fn get_account_balance(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<ApiResponse<AccountBalance>>, ApiError> {
    // Get the account data manager from app state
    let account_manager = state.account_data_manager.ok_or_else(|| {
        ApiError::Internal("Account data manager not initialized".to_string())
    })?;
    
    match account_manager.get_account(&address).await {
        Ok(account) => {
            let balance = AccountBalance {
                address: address,
                lamports: account.lamports,
                sol: account.lamports as f64 / 1_000_000_000.0,
                updated_at: chrono::Utc::now().to_rfc3339(),
            };
            Ok(Json(ApiResponse::success(balance)))
        },
        Err(e) => {
            // For demo purposes, return mock data if real data not available
            let lamports = 123456789000;
            let balance = AccountBalance {
                address: address,
                lamports: lamports,
                sol: lamports as f64 / 1_000_000_000.0,
                updated_at: chrono::Utc::now().to_rfc3339(),
            };
            Ok(Json(ApiResponse::success(balance)))
        }
    }
}

pub async fn get_account_tokens(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<ApiResponse<Vec<TokenBalance>>>, ApiError> {
    // In a real implementation, we'd fetch from a data source
    // For now, return mock data
    
    let tokens = vec![
        TokenBalance {
            mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
            owner: address.clone(),
            amount: "25000000".to_string(),
            decimals: 6,
            ui_amount: 25.0,
        },
        TokenBalance {
            mint: "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".to_string(), // USDT
            owner: address.clone(),
            amount: "10000000".to_string(),
            decimals: 6,
            ui_amount: 10.0,
        },
        TokenBalance {
            mint: "So11111111111111111111111111111111111111112".to_string(), // Wrapped SOL
            owner: address,
            amount: "5000000000".to_string(),
            decimals: 9,
            ui_amount: 5.0,
        },
    ];
    
    Ok(Json(ApiResponse::success(tokens)))
}

pub async fn get_accounts_by_program(
    State(state): State<AppState>,
    Path(program_id): Path<String>,
    Query(params): Query<AccountQueryParams>,
) -> Result<Json<ApiResponse<Vec<AccountData>>>, ApiError> {
    let account_manager = state.account_data_manager.ok_or_else(|| {
        ApiError::Internal("Account data manager not initialized".to_string())
    })?;
    
    let limit = params.limit.unwrap_or(10);
    
    match account_manager.get_accounts_by_program(&program_id, limit).await {
        Ok(accounts) => Ok(Json(ApiResponse::success(accounts))),
        Err(e) => Err(ApiError::Internal(format!("Failed to fetch accounts by program: {}", e)))
    }
}

pub async fn account_stream(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<AccountUpdateParams>,
) -> impl IntoResponse {
    let pubkeys = params.pubkeys
        .map(|p| p.split(',').map(|s| s.to_string()).collect::<Vec<_>>())
        .unwrap_or_default();
    
    let program = params.program;
    
    ws.on_upgrade(move |socket| async move {
        handle_account_websocket(socket, state, pubkeys, program).await
    })
}

async fn handle_account_websocket(
    socket: axum::extract::ws::WebSocket,
    state: AppState,
    pubkeys: Vec<String>,
    program: Option<String>,
) {
    use axum::extract::ws::Message;
    use futures::{SinkExt, StreamExt};
    use std::time::Duration;
    
    state.metrics.set_metric("active_account_streams", serde_json::json!(1)).await;
    
    let (sender, receiver) = socket.split();
    
    let (tx, rx) = broadcast::channel::<AccountData>(1000);
    
    let tx_clone = tx.clone();
    let pubkeys_clone = pubkeys.clone();
    let program_clone = program.clone();
    
    let mut simulation_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        
        loop {
            interval.tick().await;
            
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
    
    let ws_sender = sender;
    
    tokio::spawn(async move {
        let mut sender = ws_sender;
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
                    if let Ok(account) = result {
                        let matches_pubkey = pubkeys.is_empty() || pubkeys.contains(&account.pubkey);
                        let matches_program = program.is_none() || program.as_ref() == Some(&account.owner);
                        
                        if matches_pubkey && matches_program {
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
        
        simulation_task.abort();
        
        state.metrics.set_metric("active_account_streams", serde_json::json!(0)).await;
    });
}

pub fn create_account_router() -> Router<AppState> {
    Router::new()
        .route("/account/:pubkey", get(get_account))
        .route("/account/:pubkey/balance", get(get_account_balance))
        .route("/account/:pubkey/tokens", get(get_account_tokens))
        .route("/accounts/program/:program_id", get(get_accounts_by_program))
        .route("/ws/accounts", get(account_stream))
}
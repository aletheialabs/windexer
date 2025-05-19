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

// Types for block data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockData {
    pub slot: u64,
    pub parent_slot: u64,
    pub blockhash: String,
    pub previous_blockhash: String,
    pub block_time: Option<i64>,
    pub block_height: Option<u64>,
    pub transaction_count: u64,
    pub leader: String,
    pub rewards: Option<Vec<Reward>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reward {
    pub pubkey: String,
    pub lamports: i64,
    pub post_balance: u64,
    pub reward_type: Option<String>,
}

// Query parameters for blocks
#[derive(Debug, Deserialize)]
pub struct BlockQueryParams {
    pub limit: Option<usize>,
    pub before: Option<u64>,
    pub after: Option<u64>,
}

// Get block by slot
pub async fn get_block(
    State(_state): State<AppState>,
    Path(slot): Path<u64>,
) -> Result<Json<ApiResponse<BlockData>>, ApiError> {
    // For now, return placeholder data
    let block = BlockData {
        slot,
        parent_slot: slot.saturating_sub(1),
        blockhash: format!("blockhash{}", slot),
        previous_blockhash: format!("blockhash{}", slot.saturating_sub(1)),
        block_time: Some(chrono::Utc::now().timestamp()),
        block_height: Some(slot),
        transaction_count: 100,
        leader: "11111111111111111111111111111111".to_string(),
        rewards: Some(vec![
            Reward {
                pubkey: "11111111111111111111111111111111".to_string(),
                lamports: 10000,
                post_balance: 1000000000,
                reward_type: Some("fee".to_string()),
            }
        ]),
    };

    Ok(Json(ApiResponse::success(block)))
}

// Get latest block
pub async fn get_latest_block(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<BlockData>>, ApiError> {
    // For now, return placeholder data
    let latest_slot = 100000000;
    
    let block = BlockData {
        slot: latest_slot,
        parent_slot: latest_slot - 1,
        blockhash: format!("blockhash{}", latest_slot),
        previous_blockhash: format!("blockhash{}", latest_slot - 1),
        block_time: Some(chrono::Utc::now().timestamp()),
        block_height: Some(latest_slot),
        transaction_count: 100,
        leader: "11111111111111111111111111111111".to_string(),
        rewards: Some(vec![
            Reward {
                pubkey: "11111111111111111111111111111111".to_string(),
                lamports: 10000,
                post_balance: 1000000000,
                reward_type: Some("fee".to_string()),
            }
        ]),
    };

    Ok(Json(ApiResponse::success(block)))
}

// Get blocks in range
pub async fn get_blocks(
    State(_state): State<AppState>,
    Query(params): Query<BlockQueryParams>,
) -> Result<Json<ApiResponse<Vec<BlockData>>>, ApiError> {
    // For now, return placeholder data
    let limit = params.limit.unwrap_or(10);
    let latest_slot = 100000000;
    
    let start_slot = params.after.unwrap_or(latest_slot - limit as u64);
    let end_slot = params.before.unwrap_or(latest_slot);
    
    let blocks: Vec<BlockData> = (start_slot..=end_slot)
        .take(limit)
        .map(|slot| BlockData {
            slot,
            parent_slot: slot.saturating_sub(1),
            blockhash: format!("blockhash{}", slot),
            previous_blockhash: format!("blockhash{}", slot.saturating_sub(1)),
            block_time: Some(chrono::Utc::now().timestamp() - ((end_slot - slot) as i64 * 400)), // 400ms per slot
            block_height: Some(slot),
            transaction_count: fastrand::u64(10..200),
            leader: "11111111111111111111111111111111".to_string(),
            rewards: Some(vec![
                Reward {
                    pubkey: "11111111111111111111111111111111".to_string(),
                    lamports: 10000,
                    post_balance: 1000000000,
                    reward_type: Some("fee".to_string()),
                }
            ]),
        })
        .collect();

    Ok(Json(ApiResponse::success(blocks)))
}

// WebSocket handler for real-time block updates
pub async fn block_stream(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        handle_block_websocket(socket, state).await
    })
}

// Internal function to handle the WebSocket connection
async fn handle_block_websocket(
    socket: axum::extract::ws::WebSocket,
    state: AppState,
) {
    use axum::extract::ws::Message;
    use futures::{SinkExt, StreamExt};
    use std::time::Duration;
    
    // Record metric for active streams
    state.metrics.set_metric("active_block_streams", serde_json::json!(1)).await;
    
    // Split the socket into sender and receiver
    let (sender, receiver) = socket.split();
    
    // Create a channel for block updates
    let (tx, rx) = broadcast::channel::<BlockData>(100);
    
    // Spawn a task to simulate real block updates
    let tx_clone = tx.clone();
    
    let mut simulation_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(400)); // ~400ms per slot
        let mut current_slot = 100000000;
        
        loop {
            interval.tick().await;
            
            // Simulate a block update
            current_slot += 1;
            
            let block = BlockData {
                slot: current_slot,
                parent_slot: current_slot - 1,
                blockhash: format!("blockhash{}", current_slot),
                previous_blockhash: format!("blockhash{}", current_slot - 1),
                block_time: Some(chrono::Utc::now().timestamp()),
                block_height: Some(current_slot),
                transaction_count: fastrand::u64(10..200),
                leader: format!("leader{}", fastrand::u64(..10)),
                rewards: Some(vec![
                    Reward {
                        pubkey: "11111111111111111111111111111111".to_string(),
                        lamports: 10000,
                        post_balance: 1000000000,
                        reward_type: Some("fee".to_string()),
                    }
                ]),
            };
            
            let _ = tx_clone.send(block);
        }
    });
    
    // Clone the sender for separate use in our tasks
    let ws_sender = sender;
    
    // Spawn a task to handle WebSocket messages and block updates
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
                
                // Handle block updates from broadcast channel
                result = rx.recv() => {
                    if let Ok(block) = result {
                        // Serialize and send the block update
                        if let Ok(json) = serde_json::to_string(&block) {
                            if sender.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        // Cancel the simulation task when the WebSocket closes
        simulation_task.abort();
        
        // Update metric when connection ends
        state.metrics.set_metric("active_block_streams", serde_json::json!(0)).await;
    });
}

pub fn create_block_router() -> Router<AppState> {
    Router::new()
        .route("/blocks/latest", get(get_latest_block))
        .route("/blocks/:slot", get(get_block))
        .route("/blocks", get(get_blocks))
        .route("/ws/blocks", get(block_stream))
}

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

#[derive(Debug, Deserialize)]
pub struct BlockQueryParams {
    pub limit: Option<usize>,
    pub before: Option<u64>,
    pub after: Option<u64>,
}

pub async fn get_block(
    State(state): State<AppState>,
    Path(slot): Path<u64>,
) -> Result<Json<ApiResponse<BlockData>>, ApiError> {
    let helius_client = state.helius_client.as_ref().ok_or_else(|| {
        ApiError::Internal("Helius client not initialized".to_string())
    })?;
    
    match helius_client.get_block_by_slot(slot).await {
        Ok(block) => {
            tracing::debug!("Helius block for slot {}: {:?}", slot, block);
            Ok(Json(ApiResponse::success(block)))
        }
        Err(e) => {
            tracing::error!("Error fetching block {} from Helius: {}", slot, e);
            Err(ApiError::NotFound(format!("Block not found at slot {}: {}", slot, e)))
        }
    }
}

pub async fn get_latest_block(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<BlockData>>, ApiError> {
    let helius_client = state.helius_client.as_ref().ok_or_else(|| {
        ApiError::Internal("Helius client not initialized".to_string())
    })?;
    
    match helius_client.get_latest_block().await {
        Ok(block) => {
            tracing::debug!("Helius latest block: {:?}", block);
            Ok(Json(ApiResponse::success(block)))
        }
        Err(e) => {
            tracing::error!("Error fetching latest block from Helius: {}", e);
            Err(ApiError::Internal(format!("Failed to fetch latest block: {}", e)))
        }
    }
}

pub async fn get_blocks(
    State(state): State<AppState>,
    Query(params): Query<BlockQueryParams>,
) -> Result<Json<ApiResponse<Vec<BlockData>>>, ApiError> {
    let limit = params.limit.unwrap_or(10);
    
    let helius_client = state.helius_client.as_ref().ok_or_else(|| {
        ApiError::Internal("Helius client not initialized".to_string())
    })?;
    
    match helius_client.get_blocks(limit).await {
        Ok(blocks) => {
            tracing::debug!("Helius blocks: {:?}", blocks);
            Ok(Json(ApiResponse::success(blocks)))
        }
        Err(e) => {
            tracing::error!("Error fetching blocks from Helius: {}", e);
            Err(ApiError::Internal(format!("Failed to fetch blocks: {}", e)))
        }
    }
}

pub async fn block_stream(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        handle_block_websocket(socket, state).await
    })
}

async fn handle_block_websocket(
    socket: axum::extract::ws::WebSocket,
    state: AppState,
) {
    use axum::extract::ws::Message;
    use futures::{SinkExt, StreamExt};
    use std::time::Duration;
    
    state.metrics.set_metric("active_block_streams", serde_json::json!(1)).await;
    
    let (sender, receiver) = socket.split();
    
    let (tx, rx) = broadcast::channel::<BlockData>(100);
    
    let mut real_connection = false;
    if let Some(helius_client) = &state.helius_client {
        // Try to connect to Helius WebSocket
        if let Err(e) = helius_client.connect_websocket().await {
            tracing::warn!("Failed to connect to Helius WebSocket: {}", e);
        } else {
            // Try to subscribe to slot updates
            if let Err(e) = helius_client.subscribe_slot_updates().await {
                tracing::warn!("Failed to subscribe to slot updates: {}", e);
            } else {
                real_connection = true;
                tracing::info!("Successfully subscribed to Helius slot updates");
            }
        }
    }
    
    let mut simulation_task = if !real_connection {
        let tx_clone = tx.clone();
        
        tracing::info!("Using simulated block data (Helius WebSocket connection unavailable)");
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(400)); // ~400ms per slot
            let mut current_slot = 100000000;
            
            loop {
                interval.tick().await;
                
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
        })
    } else {
        // If we have a real connection, set up a task that will use Helius client to get real block data
        let helius_client = state.helius_client.as_ref().unwrap().clone(); 
        let tx_clone = tx.clone();
        
        tokio::spawn(async move {
            // This would be where we'd handle real-time WebSocket messages from Helius
            // For now, we'll poll for the latest block every second
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            let mut last_seen_slot: Option<u64> = None;
            
            loop {
                interval.tick().await;
                
                // Get the latest block
                match helius_client.get_latest_block().await {
                    Ok(block) => {
                        // Only send if it's a new slot
                        if let Some(last_slot) = last_seen_slot {
                            if block.slot <= last_slot {
                                continue;
                            }
                        }
                        
                        // Update last seen slot
                        last_seen_slot = Some(block.slot);
                        
                        // Send the block update
                        let _ = tx_clone.send(block);
                    }
                    Err(e) => {
                        tracing::error!("Error fetching latest block: {}", e);
                    }
                }
            }
        })
    };
    
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
        
        simulation_task.abort();
        
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

pub fn create_jito_compat_blocks_router() -> Router<AppState> {
    Router::new()
        .route("/blocks", get(get_blocks_jito_compat))
        .route("/blocks/:slot", get(get_block_by_slot_jito_compat))
        .route("/blocks/latest", get(get_latest_block_jito_compat))
}

async fn get_blocks_jito_compat(
    State(state): State<AppState>,
    Query(params): Query<BlockQueryParams>,
) -> Result<Json<Vec<BlockData>>, ApiError> {
    let blocks = get_blocks_internal(state, params).await?;
    Ok(Json(blocks))
}

async fn get_block_by_slot_jito_compat(
    State(state): State<AppState>,
    Path(slot): Path<u64>,
) -> Result<Json<BlockData>, ApiError> {
    let block = get_block_by_slot_internal(state, slot).await?;
    Ok(Json(block))
}

async fn get_latest_block_jito_compat(
    State(state): State<AppState>,
) -> Result<Json<BlockData>, ApiError> {
    let block = get_latest_block_internal(state).await?;
    Ok(Json(block))
}

// Internal functions to avoid code duplication
async fn get_blocks_internal(
    state: AppState,
    params: BlockQueryParams,
) -> Result<Vec<BlockData>, ApiError> {
    let limit = params.limit.unwrap_or(10).min(100);

    if let Some(helius) = &state.helius_client {
        let blocks = helius.get_blocks(limit).await
            .map_err(|e| ApiError::InternalError(format!("Failed to fetch blocks: {}", e)))?;
            
        Ok(blocks)
    } else {
        let mut blocks = Vec::new();
        for i in 0..limit {
            let slot = 100000000 + i as u64;
            blocks.push(BlockData {
                slot,
                parent_slot: slot - 1,
                blockhash: format!("blockhash{}", slot),
                previous_blockhash: format!("blockhash{}", slot - 1),
                block_time: Some(chrono::Utc::now().timestamp()),
                block_height: Some(slot),
                transaction_count: 100,
                leader: format!("leader{}", i),
                rewards: Some(vec![]),
            });
        }
        Ok(blocks)
    }
}

async fn get_block_by_slot_internal(
    state: AppState,
    slot: u64,
) -> Result<BlockData, ApiError> {
    if let Some(helius) = &state.helius_client {
        // Use Helius API to get block
        let block = helius.get_block_by_slot(slot).await
            .map_err(|e| ApiError::InternalError(format!("Failed to fetch block: {}", e)))?;
            
        Ok(block)
    } else {
        Ok(BlockData {
            slot,
            parent_slot: slot - 1,
            blockhash: format!("blockhash{}", slot),
            previous_blockhash: format!("blockhash{}", slot - 1),
            block_time: Some(chrono::Utc::now().timestamp()),
            block_height: Some(slot),
            transaction_count: 100,
            leader: format!("leader{}", slot),
            rewards: Some(vec![]),
        })
    }
}

async fn get_latest_block_internal(
    state: AppState,
) -> Result<BlockData, ApiError> {
    if let Some(helius) = &state.helius_client {
        // Use Helius API to get latest block
        helius.get_latest_block().await
            .map_err(|e| ApiError::InternalError(format!("Failed to fetch latest block: {}", e)))
    } else {
        // Return mock data for testing
        let slot = 100000000;
        Ok(BlockData {
            slot,
            parent_slot: slot - 1,
            blockhash: format!("blockhash{}", slot),
            previous_blockhash: format!("blockhash{}", slot - 1),
            block_time: Some(chrono::Utc::now().timestamp()),
            block_height: Some(slot),
            transaction_count: 100,
            leader: "leader0".to_string(),
            rewards: Some(vec![]),
        })
    }
}

//! # Manual Merge Handler
//! 
//! Handles Manual Mode sync operations: get/set sync mode, 
//! get pending ops, confirm merge, discard pending.

use std::sync::Arc;
use tokio::sync::broadcast;
use deve_core::protocol::ServerMessage;
use deve_core::config::SyncMode;
use crate::server::AppState;

/// Get current sync mode
pub async fn handle_get_sync_mode(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
) {
    let mode = {
        let engine = state.sync_engine.read().unwrap();
        engine.sync_mode()
    };
    
    let mode_str = match mode {
        SyncMode::Auto => "auto".to_string(),
        SyncMode::Manual => "manual".to_string(),
    };
    
    let _ = tx.send(ServerMessage::SyncModeStatus { mode: mode_str });
}

/// Set sync mode
pub async fn handle_set_sync_mode(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    mode: String,
) {
    let new_mode = match mode.to_lowercase().as_str() {
        "auto" => SyncMode::Auto,
        "manual" => SyncMode::Manual,
        _ => {
            let _ = tx.send(ServerMessage::Error(format!("Invalid sync mode: {}", mode)));
            return;
        }
    };
    
    {
        let mut engine = state.sync_engine.write().unwrap();
        engine.set_sync_mode(new_mode);
    }
    
    tracing::info!("SetSyncMode: {:?}", new_mode);
    
    let mode_str = match new_mode {
        SyncMode::Auto => "auto".to_string(),
        SyncMode::Manual => "manual".to_string(),
    };
    
    let _ = tx.send(ServerMessage::SyncModeStatus { mode: mode_str });
}

/// Get pending operations with diff preview
pub async fn handle_get_pending_ops(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
) {
    let pending_count = {
        let engine = state.sync_engine.read().unwrap();
        engine.pending_ops_count()
    };
    
    // Generate previews (simplified for now)
    let previews: Vec<(String, String, String)> = if pending_count > 0 {
        // TODO: Generate actual diff previews from pending ops
        vec![("(pending operations)".to_string(), "...".to_string(), "...".to_string())]
    } else {
        vec![]
    };
    
    let _ = tx.send(ServerMessage::PendingOpsInfo { 
        count: pending_count as u32, 
        previews 
    });
}

/// Confirm merge of all pending operations
pub async fn handle_confirm_merge(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
) {
    let result = {
        let mut engine = state.sync_engine.write().unwrap();
        engine.merge_pending()
    };
    
    match result {
        Ok(count) => {
            tracing::info!("Merged {} pending operations", count);
            let _ = tx.send(ServerMessage::MergeComplete { merged_count: count as u32 });
        }
        Err(e) => {
            tracing::error!("Merge failed: {:?}", e);
            let _ = tx.send(ServerMessage::Error(format!("Merge failed: {}", e)));
        }
    }
}

/// Discard all pending operations
pub async fn handle_discard_pending(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
) {
    {
        let mut engine = state.sync_engine.write().unwrap();
        engine.clear_pending();
    }
    
    tracing::info!("Discarded all pending operations");
    let _ = tx.send(ServerMessage::PendingDiscarded);
}

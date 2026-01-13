//! # 手动合并处理器 (Manual Merge Handler)
//! 
//! 处理手动同步模式 (`Manual Mode`) 的相关操作：
//! 获取/设置同步模式、获取待合并操作、确认合并、丢弃待合并操作。

use std::sync::Arc;
use tokio::sync::broadcast;
use deve_core::protocol::ServerMessage;
use deve_core::config::SyncMode;
use crate::server::AppState;

/// 获取当前同步模式
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

/// 设置同步模式
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

/// 获取待合并操作及其预览
pub async fn handle_get_pending_ops(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
) {
    let pending_count = {
        let engine = state.sync_engine.read().unwrap();
        engine.pending_ops_count()
    };
    
    // 生成预览 (目前简化处理)
    let previews: Vec<(String, String, String)> = if pending_count > 0 {
        // TODO: 从 pending ops 生成实际的 diff 预览
        vec![("(pending operations)".to_string(), "...".to_string(), "...".to_string())]
    } else {
        vec![]
    };
    
    let _ = tx.send(ServerMessage::PendingOpsInfo { 
        count: pending_count as u32, 
        previews 
    });
}

/// 确认合并所有待处理的操作
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

/// 丢弃所有待处理的操作
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

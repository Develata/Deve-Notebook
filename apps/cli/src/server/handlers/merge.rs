// apps/cli/src/server/handlers/merge.rs
//! # 手动合并处理器 (Manual Merge Handler)
//!
//! 处理手动同步模式相关操作：
//! 获取/设置同步模式、获取待合并操作、确认合并、丢弃待合并操作。

use crate::server::AppState;
use crate::server::channel::DualChannel;
use deve_core::config::SyncMode;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

/// 获取当前同步模式
pub async fn handle_get_sync_mode(state: &Arc<AppState>, ch: &DualChannel) {
    let mode = {
        let engine = state.sync_engine.read().unwrap_or_else(|e| e.into_inner());
        engine.sync_mode()
    };

    let mode_str = match mode {
        SyncMode::Auto => "auto".to_string(),
        SyncMode::Manual => "manual".to_string(),
    };

    ch.unicast(ServerMessage::SyncModeStatus { mode: mode_str });
}

/// 设置同步模式
pub async fn handle_set_sync_mode(state: &Arc<AppState>, ch: &DualChannel, mode: String) {
    let new_mode = match mode.to_lowercase().as_str() {
        "auto" => SyncMode::Auto,
        "manual" => SyncMode::Manual,
        _ => {
            ch.send_error(format!("Invalid sync mode: {}", mode));
            return;
        }
    };

    {
        let mut engine = state.sync_engine.write().unwrap_or_else(|e| e.into_inner());
        engine.set_sync_mode(new_mode);
    }

    tracing::info!("SetSyncMode: {:?}", new_mode);

    let mode_str = match new_mode {
        SyncMode::Auto => "auto".to_string(),
        SyncMode::Manual => "manual".to_string(),
    };

    ch.unicast(ServerMessage::SyncModeStatus { mode: mode_str });
}

/// 获取待合并操作及其预览
pub async fn handle_get_pending_ops(state: &Arc<AppState>, ch: &DualChannel) {
    let pending_count = {
        let engine = state.sync_engine.read().unwrap_or_else(|e| e.into_inner());
        engine.pending_ops_count()
    };

    // 生成预览 (目前简化处理)
    let previews: Vec<(String, String, String)> = if pending_count > 0 {
        vec![(
            "(pending operations)".to_string(),
            "...".to_string(),
            "...".to_string(),
        )]
    } else {
        vec![]
    };

    ch.unicast(ServerMessage::PendingOpsInfo {
        count: pending_count as u32,
        previews,
    });
}

/// 确认合并所有待处理的操作
pub async fn handle_confirm_merge(state: &Arc<AppState>, ch: &DualChannel) {
    let result = {
        let mut engine = state.sync_engine.write().unwrap_or_else(|e| e.into_inner());
        engine.merge_pending()
    };

    match result {
        Ok(count) => {
            tracing::info!("Merged {} pending operations", count);
            ch.broadcast(ServerMessage::MergeComplete {
                merged_count: count as u32,
            });
        }
        Err(e) => {
            tracing::error!("Merge failed: {:?}", e);
            ch.send_error(format!("Merge failed: {}", e));
        }
    }
}

/// 丢弃所有待处理的操作
pub async fn handle_discard_pending(state: &Arc<AppState>, ch: &DualChannel) {
    {
        let mut engine = state.sync_engine.write().unwrap_or_else(|e| e.into_inner());
        engine.clear_pending();
    }

    tracing::info!("Discarded all pending operations");
    ch.unicast(ServerMessage::PendingDiscarded);
}

/// 处理 P2P 分支合并
pub async fn handle_merge_peer(
    state: &Arc<AppState>,
    ch: &DualChannel,
    peer_id: String,
    doc_id: deve_core::models::DocId,
) {
    let repo = &state.repo;

    // 1. Get Repo ID
    let repo_id = match repo.get_repo_info() {
        Ok(Some(info)) => info.uuid,
        Ok(None) => uuid::Uuid::nil(),
        Err(e) => {
            ch.send_error(format!("Failed to get repo info: {}", e));
            return;
        }
    };

    let pid = deve_core::models::PeerId::new(peer_id);

    // 2. Perform Merge
    let result = repo.merge_peer(&pid, &repo_id, doc_id);

    match result {
        Ok(merge_res) => {
            match merge_res {
                deve_core::ledger::merge::MergeResult::Success(content) => {
                    // 3. 成功: 写入文件系统
                    if let Some(path_str) = repo.get_path_by_docid(doc_id).unwrap_or(None) {
                        let abs_path = state.vault_path.join(&path_str);

                        if let Err(e) = std::fs::write(&abs_path, &content) {
                            ch.send_error(format!("Failed to write merged content: {}", e));
                            return;
                        }

                        tracing::info!("Merge Success for doc {} ({})", doc_id, path_str);
                        ch.broadcast(ServerMessage::MergeComplete { merged_count: 1 });
                    } else {
                        ch.send_error("Doc path not found for merged document".to_string());
                    }
                }
                deve_core::ledger::merge::MergeResult::Conflict { local, remote, .. } => {
                    // 4. 冲突: 通知前端
                    tracing::warn!("Merge Conflict detected for doc {}", doc_id);

                    if let Some(path) = repo.get_path_by_docid(doc_id).unwrap_or(None) {
                        ch.unicast(ServerMessage::DocDiff {
                            path,
                            old_content: local,
                            new_content: remote,
                        });
                        ch.send_error("Merge Conflict detected. Showing Diff View.".to_string());
                    }
                }
            }
        }
        Err(e) => {
            ch.send_error(format!("Merge failed: {}", e));
        }
    }
}

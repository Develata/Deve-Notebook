//! # Source Control 处理器 (Source Control Handler)
//!
//! 处理版本控制相关的 WebSocket 请求：
//! - GetChanges: 获取暂存区/未暂存变更列表
//! - StageFile: 暂存指定文件
//! - UnstageFile: 取消暂存
//! - Commit: 创建提交
//! - GetCommitHistory: 获取提交历史

use std::sync::Arc;
use tokio::sync::broadcast;
use deve_core::protocol::ServerMessage;
use deve_core::source_control::ChangeEntry;
use deve_core::models::RepoType;
use crate::server::AppState;

/// 获取变更列表 (暂存区 + 未暂存)
pub async fn handle_get_changes(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
) {
    // 获取已暂存的文件
    let staged = match state.repo.list_staged() {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("Failed to list staged files: {:?}", e);
            let _ = tx.send(ServerMessage::Error(e.to_string()));
            return;
        }
    };

    // 检测未暂存的变更 (对比 Ledger 和快照)
    let unstaged = detect_unstaged_changes(state);

    let _ = tx.send(ServerMessage::ChangesList { staged, unstaged });
}

/// 检测未暂存的变更
fn detect_unstaged_changes(state: &Arc<AppState>) -> Vec<ChangeEntry> {
    let mut changes = Vec::new();
    
    // 获取所有文档
    let docs = match state.repo.list_docs(&RepoType::Local(uuid::Uuid::nil())) {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("Failed to list docs: {:?}", e);
            return changes;
        }
    };
    
    // 获取已暂存的路径 (排除已暂存的文件)
    let staged_paths: std::collections::HashSet<String> = state.repo
        .list_staged()
        .unwrap_or_default()
        .into_iter()
        .map(|e| e.path)
        .collect();
    
    for (doc_id, path) in docs {
        // 跳过已暂存的文件
        if staged_paths.contains(&path) {
            continue;
        }
        
        // 重建当前内容
        let current = state.repo
            .get_local_ops(doc_id)
            .ok()
            .map(|ops| {
                let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
                deve_core::state::reconstruct_content(&entries)
            });
        
        // 获取已提交快照
        let committed = state.repo
            .get_committed_content(doc_id)
            .ok()
            .flatten();
        
        // 检测变更
        if let Some(status) = state.repo.detect_change(
            committed.as_deref(),
            current.as_deref(),
        ) {
            changes.push(ChangeEntry { path, status });
        }
    }
    
    changes
}


/// 暂存指定文件
pub async fn handle_stage_file(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    path: String,
) {
    match state.repo.stage_file(&path) {
        Ok(()) => {
            tracing::info!("Staged file: {}", path);
            let _ = tx.send(ServerMessage::StageAck { path });
        }
        Err(e) => {
            tracing::error!("Failed to stage file: {:?}", e);
            let _ = tx.send(ServerMessage::Error(e.to_string()));
        }
    }
}

/// 取消暂存指定文件
pub async fn handle_unstage_file(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    path: String,
) {
    match state.repo.unstage_file(&path) {
        Ok(()) => {
            tracing::info!("Unstaged file: {}", path);
            let _ = tx.send(ServerMessage::UnstageAck { path });
        }
        Err(e) => {
            tracing::error!("Failed to unstage file: {:?}", e);
            let _ = tx.send(ServerMessage::Error(e.to_string()));
        }
    }
}

/// 创建提交 (保存快照)
pub async fn handle_commit(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    message: String,
) {
    // 创建内容获取闭包
    let get_content = |path: &str| -> Option<(deve_core::models::DocId, String)> {
        // 通过路径获取 DocId
        let doc_id = state.repo.get_docid(path).ok()??;
        
        // 重建当前内容
        let ops = state.repo.get_local_ops(doc_id).ok()?;
        let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
        let content = deve_core::state::reconstruct_content(&entries);
        
        Some((doc_id, content))
    };
    
    match state.repo.create_commit_with_snapshots(&message, get_content) {
        Ok(info) => {
            tracing::info!("Created commit: {} - {}", info.id, info.message);
            let _ = tx.send(ServerMessage::CommitAck {
                commit_id: info.id,
                timestamp: info.timestamp,
            });
        }
        Err(e) => {
            tracing::error!("Failed to create commit: {:?}", e);
            let _ = tx.send(ServerMessage::Error(e.to_string()));
        }
    }
}

/// 获取提交历史
pub async fn handle_get_commit_history(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    limit: u32,
) {
    match state.repo.list_commits(limit) {
        Ok(commits) => {
            tracing::info!("Returning {} commits", commits.len());
            let _ = tx.send(ServerMessage::CommitHistory { commits });
        }
        Err(e) => {
            tracing::error!("Failed to get commit history: {:?}", e);
            let _ = tx.send(ServerMessage::Error(e.to_string()));
        }
    }
}

/// 获取文档的 Diff (已提交版本 vs 当前版本)
pub async fn handle_get_doc_diff(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    path: String,
) {
    // 获取 DocId
    let doc_id = match state.repo.get_docid(&path) {
        Ok(Some(id)) => id,
        Ok(None) => {
            let _ = tx.send(ServerMessage::Error(format!("Document not found: {}", path)));
            return;
        }
        Err(e) => {
            let _ = tx.send(ServerMessage::Error(e.to_string()));
            return;
        }
    };
    
    // 获取已提交内容 (快照)
    let old_content = state.repo
        .get_committed_content(doc_id)
        .ok()
        .flatten()
        .unwrap_or_default();
    
    // 获取当前内容 (从 Ledger 重建)
    let new_content = state.repo
        .get_local_ops(doc_id)
        .ok()
        .map(|ops| {
            let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
            deve_core::state::reconstruct_content(&entries)
        })
        .unwrap_or_default();
    
    let _ = tx.send(ServerMessage::DocDiff {
        path,
        old_content,
        new_content,
    });
}


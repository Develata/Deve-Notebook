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

    // TODO: 检测未暂存的变更 (对比 Ledger 和磁盘文件)
    // 目前简化为空列表，后续可扩展
    let unstaged: Vec<ChangeEntry> = vec![];

    let _ = tx.send(ServerMessage::ChangesList { staged, unstaged });
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

/// 创建提交
pub async fn handle_commit(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    message: String,
) {
    match state.repo.create_commit(&message) {
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

// apps/cli/src/server/handlers/source_control.rs
//! # Source Control 处理器
//!
//! 处理版本控制请求: GetChanges, StageFile, UnstageFile, Commit, GetCommitHistory

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::RepoType;
use deve_core::protocol::ServerMessage;
use deve_core::source_control::ChangeEntry;
use std::sync::Arc;

/// 获取变更列表 (暂存区 + 未暂存)
///
/// 使用 session 上下文确定当前仓库
pub async fn handle_get_changes(state: &Arc<AppState>, ch: &DualChannel, session: &WsSession) {
    // 只读模式没有暂存/未暂存概念，返回空列表
    if session.is_readonly() {
        ch.unicast(ServerMessage::ChangesList {
            staged: vec![],
            unstaged: vec![],
        });
        return;
    }

    let staged = match state.repo.list_staged() {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("Failed to list staged files: {:?}", e);
            ch.send_error(e.to_string());
            return;
        }
    };

    let unstaged = detect_unstaged_changes(state, session);

    ch.unicast(ServerMessage::ChangesList { staged, unstaged });
}

/// 检测未暂存的变更
///
/// 使用 session 的 active_db 或回退到 active_repo
fn detect_unstaged_changes(state: &Arc<AppState>, session: &WsSession) -> Vec<ChangeEntry> {
    let mut changes = Vec::new();

    // 使用 session 锁定的数据库，或回退到默认逻辑
    let docs = if let Some(handle) = session.get_active_db() {
        deve_core::ledger::metadata::list_docs(&handle.db)
    } else {
        let repo_id = super::get_repo_id(state);
        state.repo.list_docs(&RepoType::Local(repo_id))
    };

    let docs = match docs {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("Failed to list docs: {:?}", e);
            return changes;
        }
    };

    let staged_paths: std::collections::HashSet<String> = state
        .repo
        .list_staged()
        .unwrap_or_default()
        .into_iter()
        .map(|e| deve_core::utils::path::to_forward_slash(&e.path))
        .collect();

    for (doc_id, path) in docs {
        let normalized = deve_core::utils::path::to_forward_slash(&path);
        if staged_paths.contains(&normalized) {
            continue;
        }

        let current = state.repo.get_local_ops(doc_id).ok().map(|ops| {
            let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
            deve_core::state::reconstruct_content(&entries)
        });

        let committed = state.repo.get_committed_content(doc_id).ok().flatten();

        if let Some(status) = state
            .repo
            .detect_change(committed.as_deref(), current.as_deref())
        {
            changes.push(ChangeEntry { path, status });
        }
    }

    changes
}

/// 暂存指定文件
pub async fn handle_stage_file(state: &Arc<AppState>, ch: &DualChannel, path: String) {
    let path = deve_core::utils::path::to_forward_slash(&path);
    match state.repo.stage_file(&path) {
        Ok(()) => {
            tracing::info!("Staged file: {}", path);
            ch.unicast(ServerMessage::StageAck { path });
        }
        Err(e) => {
            tracing::error!("Failed to stage file: {:?}", e);
            ch.send_error(e.to_string());
        }
    }
}

/// 取消暂存指定文件
pub async fn handle_unstage_file(state: &Arc<AppState>, ch: &DualChannel, path: String) {
    let path = deve_core::utils::path::to_forward_slash(&path);
    match state.repo.unstage_file(&path) {
        Ok(()) => {
            tracing::info!("Unstaged file: {}", path);
            ch.unicast(ServerMessage::UnstageAck { path });
        }
        Err(e) => {
            tracing::error!("Failed to unstage file: {:?}", e);
            ch.send_error(e.to_string());
        }
    }
}

/// 创建提交 (保存快照)
pub async fn handle_commit(state: &Arc<AppState>, ch: &DualChannel, message: String) {
    let get_content = |path: &str| -> Option<(deve_core::models::DocId, String)> {
        let normalized = deve_core::utils::path::to_forward_slash(path);
        let doc_id = state.repo.get_docid(&normalized).ok()??;
        let ops = state.repo.get_local_ops(doc_id).ok()?;
        let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
        let content = deve_core::state::reconstruct_content(&entries);
        Some((doc_id, content))
    };

    match state
        .repo
        .create_commit_with_snapshots(&message, get_content)
    {
        Ok(info) => {
            tracing::info!("Created commit: {} - {}", info.id, info.message);
            // 广播提交成功 (其他标签页需要更新)
            ch.broadcast(ServerMessage::CommitAck {
                commit_id: info.id,
                timestamp: info.timestamp,
            });
        }
        Err(e) => {
            tracing::error!("Failed to create commit: {:?}", e);
            ch.send_error(e.to_string());
        }
    }
}

/// 获取提交历史
pub async fn handle_get_commit_history(state: &Arc<AppState>, ch: &DualChannel, limit: u32) {
    match state.repo.list_commits(limit) {
        Ok(commits) => {
            tracing::info!("Returning {} commits", commits.len());
            ch.unicast(ServerMessage::CommitHistory { commits });
        }
        Err(e) => {
            tracing::error!("Failed to get commit history: {:?}", e);
            ch.send_error(e.to_string());
        }
    }
}

/// 获取文档的 Diff
///
/// **Local 分支**: 已提交版本 (左) vs 当前版本 (右)
/// **Remote 分支**: Local 对应文档 (左) vs Remote 文档 (右)
pub async fn handle_get_doc_diff(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    path: String,
) {
    // 只读模式 (Remote 分支) → 跨分支 diff
    if session.is_readonly() {
        handle_remote_diff(state, ch, session, path).await;
        return;
    }

    // Local 分支 → 原有逻辑 (committed vs current)
    let doc_id = match state.repo.get_docid(&path) {
        Ok(Some(id)) => id,
        Ok(None) => {
            ch.send_error(format!("Document not found: {}", path));
            return;
        }
        Err(e) => {
            ch.send_error(e.to_string());
            return;
        }
    };

    let old_content = state
        .repo
        .get_committed_content(doc_id)
        .ok()
        .flatten()
        .unwrap_or_default();

    let new_content = state
        .repo
        .get_local_ops(doc_id)
        .ok()
        .map(|ops| {
            let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
            deve_core::state::reconstruct_content(&entries)
        })
        .unwrap_or_default();

    ch.unicast(ServerMessage::DocDiff {
        path,
        old_content,
        new_content,
    });
}

/// Remote 分支的跨分支 Diff
///
/// **左侧 (old)**: Local 分支对应文档
/// **右侧 (new)**: Remote 分支文档 (当前只读视图)
async fn handle_remote_diff(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    path: String,
) {
    // 从 Remote DB 获取文档内容 (右侧)
    let new_content = match get_remote_doc_content(session, &path) {
        Some(content) => content,
        None => {
            ch.send_error(format!("Remote document not found: {}", path));
            return;
        }
    };

    // 从 Local 分支获取对应文档 (左侧)
    let old_content = get_local_counterpart(state, &path);

    ch.unicast(ServerMessage::DocDiff {
        path,
        old_content,
        new_content,
    });
}

/// 从 Remote DB 读取文档内容
fn get_remote_doc_content(session: &WsSession, path: &str) -> Option<String> {
    let db = session.get_active_db()?;
    let doc_id = deve_core::ledger::metadata::get_docid(&db.db, path).ok()??;
    let ops = deve_core::ledger::ops::get_ops_from_db(&db.db, doc_id).ok()?;
    let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
    Some(deve_core::state::reconstruct_content(&entries))
}

/// 从 Local 分支读取对应路径的文档
fn get_local_counterpart(state: &Arc<AppState>, path: &str) -> String {
    let doc_id = state.repo.get_docid(path).ok().flatten();
    if let Some(id) = doc_id {
        state
            .repo
            .get_local_ops(id)
            .ok()
            .map(|ops| {
                let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
                deve_core::state::reconstruct_content(&entries)
            })
            .unwrap_or_default()
    } else {
        // Local 不存在该文档，返回空 (表示 Remote 新增)
        String::new()
    }
}

/// 放弃文件变更 (恢复到已提交状态)
///
/// **逻辑**:
/// 1. 获取文档的已提交快照内容
/// 2. 清除当前 Ledger 操作并重建为快照内容
/// 3. 发送确认消息
pub async fn handle_discard_file(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    path: String,
) {
    let doc_id = match state.repo.get_docid(&path) {
        Ok(Some(id)) => id,
        Ok(None) => {
            ch.send_error(format!("Document not found: {}", path));
            return;
        }
        Err(e) => {
            ch.send_error(e.to_string());
            return;
        }
    };

    // 获取已提交的快照内容
    let committed_content = state
        .repo
        .get_committed_content(doc_id)
        .ok()
        .flatten()
        .unwrap_or_default();

    // 实际恢复逻辑:
    // 1. 获取当前内容
    // 2. 计算差异 (current -> committed)
    // 3. 应用差异操作
    let current_content = state
        .repo
        .get_local_ops(doc_id)
        .ok()
        .map(|ops| {
            let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
            deve_core::state::reconstruct_content(&entries)
        })
        .unwrap_or_default();

    if current_content == committed_content {
        tracing::info!("Discard file: {} - already matches committed state", path);
        ch.unicast(ServerMessage::DiscardAck { path: path.clone() });
        handle_get_changes(state, ch, session).await;
        return;
    }

    // 计算差异并生成反向操作
    let ops = deve_core::state::compute_diff(&current_content, &committed_content);

    // 应用操作到 Ledger
    for op in ops {
        let peer_id = deve_core::models::PeerId("local".to_string());
        // 使用 SyncManager 应用 Op，但不每次都持久化 (为了性能)
        if let Err(e) = state.sync_manager.apply_local_op(
            doc_id,
            peer_id.clone(),
            move |seq| deve_core::models::LedgerEntry {
                doc_id,
                peer_id: deve_core::models::PeerId("local".to_string()),
                seq,
                op: op.clone(),
                timestamp: chrono::Utc::now().timestamp_millis(),
            },
            false, // 稍后一次性持久化
        ) {
            tracing::error!("Failed to apply discard op: {:?}", e);
            ch.send_error(format!("Failed to discard: {}", e));
            return;
        }
    }

    // 统一持久化到 Vault
    if let Err(e) = state.sync_manager.persist_doc(doc_id) {
        tracing::error!("Failed to persist discarded content: {:?}", e);
        ch.send_error(format!("Failed to persist discard: {}", e));
        return;
    }

    tracing::info!(
        "Discard file: {} (restored to {} bytes, was {} bytes)",
        path,
        committed_content.len(),
        current_content.len()
    );

    ch.unicast(ServerMessage::DiscardAck { path: path.clone() });
    // 刷新 Changes 列表
    handle_get_changes(state, ch, session).await;
}

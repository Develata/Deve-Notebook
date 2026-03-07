use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::run_on_resolved_local_repo;
use crate::server::session::WsSession;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

/// 创建提交 (保存快照)
pub async fn handle_commit(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    message: String,
) {
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return ch.send_error(e.to_string()),
    };
    match run_on_resolved_local_repo(state, &scope, |db| {
        deve_core::ledger::source_control::create_commit(db, &message, |path| {
            let normalized = deve_core::utils::path::to_forward_slash(path);
            let doc_id = deve_core::ledger::metadata::get_docid(db, &normalized).ok()??;
            let ops = deve_core::ledger::ops::get_ops_from_db(db, doc_id).ok()?;
            let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
            Some((doc_id, deve_core::state::reconstruct_content(&entries)))
        })
    }) {
        Ok(info) => {
            tracing::info!("Created commit: {} - {}", info.id, info.message);
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
pub async fn handle_get_commit_history(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    limit: u32,
) {
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return ch.send_error(e.to_string()),
    };
    match run_on_resolved_local_repo(state, &scope, |db| {
        deve_core::ledger::source_control::list_commits(db, limit)
    }) {
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

/// 获取两个提交之间的差异
pub async fn handle_get_commit_diff(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    commit_a: Option<String>,
    commit_b: String,
) {
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return ch.send_error(e.to_string()),
    };
    match run_on_resolved_local_repo(state, &scope, |db| {
        deve_core::source_control::commit_diff::compare_commits(db, commit_a.as_deref(), &commit_b)
    }) {
        Ok(diffs) => {
            tracing::info!("Returning diff with {} file changes", diffs.len());
            ch.unicast(ServerMessage::CommitDiffResult { diffs });
        }
        Err(e) => {
            tracing::error!("Failed to get commit diff: {:?}", e);
            ch.send_error(e.to_string());
        }
    }
}
/// 提交并推送到所有已连接的 Peer
///
/// **流程**: 创建本地提交，成功后广播 CommitAck。
/// P2P 同步由现有的 SyncHello 握手机制自动触发。
pub async fn handle_commit_and_push(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    message: String,
) {
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return ch.send_error(e.to_string()),
    };
    match run_on_resolved_local_repo(state, &scope, |db| {
        deve_core::ledger::source_control::create_commit(db, &message, |path| {
            let normalized = deve_core::utils::path::to_forward_slash(path);
            let doc_id = deve_core::ledger::metadata::get_docid(db, &normalized).ok()??;
            let ops = deve_core::ledger::ops::get_ops_from_db(db, doc_id).ok()?;
            let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
            Some((doc_id, deve_core::state::reconstruct_content(&entries)))
        })
    }) {
        Ok(info) => {
            tracing::info!("Commit & Push: {} - {}", info.id, info.message);
            ch.broadcast(ServerMessage::CommitAck {
                commit_id: info.id,
                timestamp: info.timestamp,
            });
        }
        Err(e) => {
            tracing::error!("Commit & Push failed: {:?}", e);
            ch.send_error(e.to_string());
        }
    }
}

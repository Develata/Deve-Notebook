use crate::server::AppState;
use crate::server::channel::DualChannel;
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
    let selector = super::service::selector_from_scope(&scope);
    match super::service::commit_staged(state.repo.as_ref(), &selector, &message) {
        Ok(info) => {
            tracing::info!("Created commit: {} - {}", info.id, info.message);
            ch.broadcast(ServerMessage::CommitAck {
                repo_id: Some(scope.repo_id),
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
    let selector = super::service::selector_from_scope(&scope);
    match super::service::list_commit_history(state.repo.as_ref(), &selector, limit) {
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
    let selector = super::service::selector_from_scope(&scope);
    match super::service::diff_commits(
        state.repo.as_ref(),
        &selector,
        commit_a.as_deref(),
        &commit_b,
    ) {
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
    let selector = super::service::selector_from_scope(&scope);
    match super::service::commit_staged(state.repo.as_ref(), &selector, &message) {
        Ok(info) => {
            tracing::info!("Commit & Push: {} - {}", info.id, info.message);
            ch.broadcast(ServerMessage::CommitAck {
                repo_id: Some(scope.repo_id),
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

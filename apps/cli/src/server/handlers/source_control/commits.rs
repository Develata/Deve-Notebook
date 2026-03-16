use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::ledger::source_control as ledger_source_control;
use deve_core::protocol::ServerMessage;
use deve_core::source_control;
use std::sync::Arc;

/// 创建提交 (保存快照)
pub async fn handle_commit(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    message: String,
) {
    let scope_nonce = Some(session.scope_nonce());
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws(ch, e),
    };
    let selector = super::service::selector_from_scope(&scope);
    match super::service::commit_staged(state.repo.as_ref(), &selector, &message) {
        Ok(info) => {
            tracing::info!("Created commit: {} - {}", info.id, info.message);
            ch.broadcast(ServerMessage::CommitAck {
                repo_id: Some(scope.repo_id),
                branch: scope.branch.clone(),
                scope_nonce,
                commit_id: info.id,
                timestamp: info.timestamp,
            });
        }
        Err(e) => {
            tracing::error!("Failed to create commit: {:?}", e);
            super::errors::send_ws(ch, e);
        }
    }
}

/// 获取提交历史
pub async fn handle_get_commit_history(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
    limit: u32,
) {
    let scope_nonce = Some(session.scope_nonce());
    let scope = match super::repo_scope::resolve_current_repo_scope(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws(ch, e),
    };
    match list_commit_history(state, &scope, limit) {
        Ok(commits) => {
            tracing::info!("Returning {} commits", commits.len());
            ch.unicast(ServerMessage::CommitHistory {
                request_id: Some(request_id),
                repo_id: Some(scope.repo_id),
                branch: scope.branch.clone(),
                scope_nonce,
                commits,
            });
        }
        Err(e) => {
            tracing::error!("Failed to get commit history: {:?}", e);
            super::errors::send_ws(ch, e);
        }
    }
}

/// 获取两个提交之间的差异
pub async fn handle_get_commit_diff(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
    commit_a: Option<String>,
    commit_b: String,
) {
    let scope_nonce = Some(session.scope_nonce());
    let scope = match super::repo_scope::resolve_current_repo_scope(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws(ch, e),
    };
    match diff_commits(state, &scope, commit_a.as_deref(), &commit_b) {
        Ok(diffs) => {
            tracing::info!("Returning diff with {} file changes", diffs.len());
            ch.unicast(ServerMessage::CommitDiffResult {
                request_id: Some(request_id),
                repo_id: Some(scope.repo_id),
                branch: scope.branch.clone(),
                scope_nonce,
                diffs,
            });
        }
        Err(e) => {
            tracing::error!("Failed to get commit diff: {:?}", e);
            super::errors::send_ws(ch, e);
        }
    }
}

fn list_commit_history(
    state: &Arc<AppState>,
    scope: &crate::server::repo_scope::ResolvedRepo,
    limit: u32,
) -> super::service::ScResult<Vec<deve_core::source_control::CommitInfo>> {
    if let Some(peer_id) = &scope.branch {
        return state
            .repo
            .run_on_shadow_repo_by_id(peer_id, &scope.repo_id, |db| {
                ledger_source_control::list_commits(db, limit)
            })
            .map_err(|e| super::errors::map_repo_error(super::errors::ScOp::CommitHistory, e));
    }
    let selector = super::service::selector_from_scope(scope);
    super::service::list_commit_history(state.repo.as_ref(), &selector, limit)
}

fn diff_commits(
    state: &Arc<AppState>,
    scope: &crate::server::repo_scope::ResolvedRepo,
    commit_a: Option<&str>,
    commit_b: &str,
) -> super::service::ScResult<Vec<deve_core::source_control::CommitFileDiff>> {
    if let Some(peer_id) = &scope.branch {
        return state
            .repo
            .run_on_shadow_repo_by_id(peer_id, &scope.repo_id, |db| {
                source_control::commit_diff::compare_commits(db, commit_a, commit_b)
            })
            .map_err(|e| {
                super::errors::map_repo_error(
                    super::errors::ScOp::CommitDiff(commit_b.to_string()),
                    e,
                )
            });
    }
    let selector = super::service::selector_from_scope(scope);
    super::service::diff_commits(state.repo.as_ref(), &selector, commit_a, commit_b)
}

/// 提交并推送到所有已连接的 Peer
pub async fn handle_commit_and_push(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    message: String,
) {
    let scope_nonce = Some(session.scope_nonce());
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws(ch, e),
    };
    let selector = super::service::selector_from_scope(&scope);
    match super::service::commit_staged(state.repo.as_ref(), &selector, &message) {
        Ok(info) => {
            tracing::info!("Commit & Push: {} - {}", info.id, info.message);
            ch.broadcast(ServerMessage::CommitAck {
                repo_id: Some(scope.repo_id),
                branch: scope.branch.clone(),
                scope_nonce,
                commit_id: info.id,
                timestamp: info.timestamp,
            });
        }
        Err(e) => {
            tracing::error!("Commit & Push failed: {:?}", e);
            super::errors::send_ws(ch, e);
        }
    }
}

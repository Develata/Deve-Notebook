//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!
//! Source-control commit handlers.

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

/// 创建提交 (保存快照)
pub async fn handle_commit(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    message: String,
) {
    super::commits_write::commit_with_ack(
        state,
        ch,
        session,
        message,
        "Created commit",
        "Failed to create commit",
    )
    .await;
}

/// 获取提交历史
pub async fn handle_get_commit_history(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
    limit: u32,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope = match super::repo_scope::resolve_current_repo_scope(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
    };
    match super::commits_query::list_commit_history(state, &scope, limit) {
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
            super::errors::send_ws_scoped(ch, e, scope_nonce);
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
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope = match super::repo_scope::resolve_current_repo_scope(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
    };
    match super::commits_query::diff_commits(state, &scope, commit_a.as_deref(), &commit_b) {
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
            super::errors::send_ws_scoped(ch, e, scope_nonce);
        }
    }
}

/// 提交并推送到所有已连接的 Peer
pub async fn handle_commit_and_push(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    message: String,
) {
    super::commits_write::commit_with_ack(
        state,
        ch,
        session,
        message,
        "Commit & Push",
        "Commit & Push failed",
    )
    .await;
}

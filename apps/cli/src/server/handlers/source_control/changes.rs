use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::ServerMessage;
use deve_core::source_control::ChangeEntry;
use deve_core::source_control::SourceControlApi;
use std::sync::Arc;

pub async fn handle_get_changes(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: Option<String>,
) {
    let scope_nonce = Some(session.scope_nonce());
    if session.active_branch.is_some() {
        let scope = match super::repo_scope::resolve_current_repo_scope(state, session) {
            Ok(scope) => scope,
            Err(e) => return super::errors::send_ws(ch, e),
        };
        ch.unicast(ServerMessage::ChangesList {
            request_id,
            repo_id: Some(scope.repo_id),
            branch: scope.branch,
            scope_nonce,
            staged: vec![],
            unstaged: vec![],
        });
        return;
    }
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws(ch, e),
    };
    let selector = super::service::selector_from_scope(&scope);
    let staged = match state.repo.list_staged_in_repo(&selector) {
        Ok(list) => super::present::collapse_rename_candidates(list),
        Err(e) => {
            tracing::error!("Failed to list staged files: {:?}", e);
            return super::errors::send_ws(
                ch,
                super::errors::map_repo_error(super::errors::ScOp::ListChanges, e),
            );
        }
    };
    let unstaged = match detect_unstaged_changes(state, &scope) {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("Failed to list unstaged files: {:?}", e);
            return super::errors::send_ws(ch, e);
        }
    };
    ch.unicast(ServerMessage::ChangesList {
        request_id,
        repo_id: Some(scope.repo_id),
        branch: scope.branch.clone(),
        scope_nonce,
        staged,
        unstaged,
    });
}

/// 检测未暂存的变更
///
/// **Invariant**: pending_fs_ops 是当前本地 repo 的 Working Directory 单一事实源。
fn detect_unstaged_changes(
    state: &Arc<AppState>,
    scope: &crate::server::repo_scope::ResolvedRepo,
) -> super::service::ScResult<Vec<ChangeEntry>> {
    let selector = super::service::selector_from_scope(scope);
    let pending = super::service::list_pending(state.repo.as_ref(), &selector)?;

    let staged_paths: std::collections::HashSet<String> = state
        .repo
        .list_staged_in_repo(&selector)
        .map_err(|e| super::errors::map_repo_error(super::errors::ScOp::ListChanges, e))?
        .into_iter()
        .map(|entry| deve_core::utils::path::to_forward_slash(&entry.path))
        .collect();

    Ok(pending
        .into_iter()
        .filter(|e| !staged_paths.contains(&deve_core::utils::path::to_forward_slash(&e.path)))
        .collect())
}

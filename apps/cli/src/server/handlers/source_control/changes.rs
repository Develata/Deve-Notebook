use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::ServerMessage;
use deve_core::source_control::{ChangeEntry, ChangeStatus};
use deve_core::source_control::SourceControlApi;
use std::sync::Arc;

pub async fn handle_get_changes(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: Option<String>,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    if session.active_branch.is_some() {
        let scope = match super::repo_scope::resolve_current_repo_scope(state, session) {
            Ok(scope) => scope,
            Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
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
        Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
    };
    let selector = super::service::selector_from_scope(&scope);
    let staged = match state.repo.list_staged_in_repo(&selector) {
        Ok(list) => super::present::collapse_rename_candidates(list),
        Err(e) => {
            tracing::error!("Failed to list staged files: {:?}", e);
            return super::errors::send_ws_scoped(
                ch,
                super::errors::map_repo_error(super::errors::ScOp::ListChanges, e),
                scope_nonce,
            );
        }
    };
    let unstaged = match detect_unstaged_changes(state, &scope) {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("Failed to list unstaged files: {:?}", e);
            return super::errors::send_ws_scoped(ch, e, scope_nonce);
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

    let staged_keys: std::collections::HashSet<_> = state
        .repo
        .list_staged_in_repo(&selector)
        .map_err(|e| super::errors::map_repo_error(super::errors::ScOp::ListChanges, e))?
        .into_iter()
        .map(|entry| change_identity_key(&entry))
        .collect();

    Ok(pending
        .into_iter()
        .filter(|entry| !staged_keys.contains(&change_identity_key(entry)))
        .collect())
}

fn change_identity_key(
    entry: &ChangeEntry,
) -> (
    Option<deve_core::models::DocId>,
    String,
    Option<String>,
    ChangeStatus,
) {
    (
        entry.doc_id,
        deve_core::utils::path::to_forward_slash(&entry.path),
        entry
            .renamed_from
            .as_deref()
            .map(deve_core::utils::path::to_forward_slash),
        entry.status,
    )
}

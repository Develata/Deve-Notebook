use crate::server::AppState;
use crate::server::repo_scope::{ResolvedRepo, map_repo_scope_error, resolve_session_repo};
use crate::server::session::WsSession;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

pub(super) struct CurrentBranchSwitchContext {
    pub(super) scope: Option<ResolvedRepo>,
    pub(super) repo_url: Option<String>,
}

pub(super) fn resolve_current_branch_switch_context(
    state: &Arc<AppState>,
    session: &WsSession,
) -> Result<CurrentBranchSwitchContext, ServerError> {
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) => Some(scope),
        Err(err) => {
            let mapped = map_repo_scope_error(anyhow::anyhow!(err.to_string()));
            if matches!(
                mapped.code,
                ServerErrorCode::StorageNotFound
                    | ServerErrorCode::SyncRepoUnbound
                    | ServerErrorCode::ScRepoContextInvalid
            ) {
                None
            } else {
                return Err(mapped);
            }
        }
    };
    let repo_url = match scope.as_ref() {
        Some(scope) => state
            .repo
            .get_repo_url(scope.branch.as_ref(), &scope.repo_name)
            .map_err(|err| {
                map_repo_scope_error(anyhow::anyhow!(
                    "Failed to resolve current repo URL before branch switch: {}",
                    err
                ))
            })?,
        None => None,
    };
    Ok(CurrentBranchSwitchContext { scope, repo_url })
}

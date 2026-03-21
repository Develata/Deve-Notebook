use crate::server::repo_scope::{ResolvedRepo, resolve_session_repo_and_sync};
use crate::server::{AppState, session::WsSession};
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

pub fn resolve_current_repo_scope(
    state: &Arc<AppState>,
    session: &mut WsSession,
) -> Result<ResolvedRepo, ServerError> {
    if session.active_repo.is_none() && session.active_repo_id.is_none() {
        if session.active_branch.is_some() || session.has_runtime_scope_binding() {
            session.clear_active_db();
            session.clear_sync_binding();
        }
        return Err(ServerError::new(ServerErrorCode::ScRepoNotSelected));
    }
    resolve_session_repo_and_sync(state, session).map_err(super::errors::map_repo_scope_error)
}

pub fn resolve_current_local_repo(
    state: &Arc<AppState>,
    session: &mut WsSession,
) -> Result<ResolvedRepo, ServerError> {
    let scope = resolve_current_repo_scope(state, session)?;
    if scope.branch.is_some() {
        return Err(ServerError::with_detail(
            ServerErrorCode::ScRemoteBranchReadonly,
            scope.repo_name,
        ));
    }
    Ok(scope)
}

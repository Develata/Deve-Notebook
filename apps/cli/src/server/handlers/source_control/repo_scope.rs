use crate::server::repo_scope::{ResolvedRepo, resolve_session_repo};
use crate::server::{AppState, session::WsSession};
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

pub fn resolve_current_local_repo(
    state: &Arc<AppState>,
    session: &WsSession,
) -> Result<ResolvedRepo, ServerError> {
    if session.active_repo.is_none() && session.active_repo_id.is_none() {
        return Err(ServerError::new(ServerErrorCode::ScRepoNotSelected));
    }
    let scope =
        resolve_session_repo(state, session).map_err(super::errors::map_repo_scope_error)?;
    if scope.branch.is_some() {
        return Err(ServerError::with_detail(
            ServerErrorCode::ScRemoteBranchReadonly,
            scope.repo_name,
        ));
    }
    Ok(scope)
}

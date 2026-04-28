//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 06_repository#repo-scope-runtime

use crate::server::repo_scope::{
    ResolvedRepo, bootstrap_local_repo, resolve_session_repo_and_sync,
    stale_unbound_remote_scope_detail,
};
use crate::server::shadow_scope;
use crate::server::{AppState, session::WsSession};
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

pub fn resolve_current_repo_scope(
    state: &Arc<AppState>,
    session: &mut WsSession,
) -> Result<ResolvedRepo, ServerError> {
    if session.active_repo.is_none() && session.active_repo_id.is_none() {
        if let Some(branch) = session.active_branch.as_ref()
            && let Err(error) = shadow_scope::map_remote_branch_availability(state, branch)
        {
            if shadow_scope::should_clear_missing_remote_branch(&error) {
                shadow_scope::clear_stale_remote_branch(session);
            } else {
                session.clear_active_db();
                session.clear_sync_binding();
            }
            return Err(error);
        }
        if session.active_branch.is_some() && session.has_runtime_scope_binding() {
            session.clear_active_db();
            session.clear_sync_binding();
            return Err(ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                stale_unbound_remote_scope_detail(
                    session
                        .active_branch
                        .as_ref()
                        .expect("checked active branch"),
                ),
            ));
        }
        if session.active_branch.is_some() {
            session.clear_active_db();
            session.clear_sync_binding();
            return Err(ServerError::new(ServerErrorCode::ScRepoNotSelected));
        }
        if session.has_runtime_scope_binding() {
            session.clear_active_db();
            session.clear_sync_binding();
        }
        let scope =
            bootstrap_local_repo(state, session).map_err(super::errors::map_repo_scope_error)?;
        session.switch_repo(scope.repo_name.clone(), Some(scope.repo_id));
        return Ok(scope);
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

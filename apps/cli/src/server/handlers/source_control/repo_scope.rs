//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime

use crate::server::repo_scope::{
    ResolvedRepo, bootstrap_local_repo, ensure_resolved_local_repo_writable,
    resolve_session_repo_and_sync, stale_unbound_remote_scope_detail,
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
                revoke_source_control_write_grant(state, session);
                return Err(ServerError {
                    code: ServerErrorCode::ScRepoContextInvalid,
                    detail: error.detail,
                });
            } else {
                clear_runtime_binding_and_revoke(state, session);
            }
            return Err(error);
        }
        if let Some(branch) = session.active_branch.clone()
            && session.has_runtime_scope_binding()
        {
            clear_runtime_binding_and_revoke(state, session);
            return Err(ServerError::with_detail(
                ServerErrorCode::ScStaleScope,
                stale_unbound_remote_scope_detail(&branch),
            ));
        }
        if session.active_branch.is_some() {
            clear_runtime_binding_and_revoke(state, session);
            return Err(ServerError::new(ServerErrorCode::ScRepoNotSelected));
        }
        if session.has_runtime_scope_binding() {
            clear_runtime_binding_and_revoke(state, session);
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

pub fn resolve_current_writable_local_repo(
    state: &Arc<AppState>,
    session: &mut WsSession,
) -> Result<ResolvedRepo, ServerError> {
    let scope = resolve_current_local_repo(state, session)?;
    ensure_resolved_local_repo_writable(state, &scope)?;
    Ok(scope)
}

fn clear_runtime_binding_and_revoke(state: &Arc<AppState>, session: &mut WsSession) {
    session.clear_active_db();
    session.clear_sync_binding();
    revoke_source_control_write_grant(state, session);
}

fn revoke_source_control_write_grant(state: &Arc<AppState>, session: &WsSession) {
    if let Some(auth_session_id) = session.auth_session_id() {
        state
            .source_control_write_grants()
            .revoke_session(auth_session_id);
    }
}

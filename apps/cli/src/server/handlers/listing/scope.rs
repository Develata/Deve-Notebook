//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{
    RepoScopeFailure, map_repo_scope_error, stale_unbound_remote_scope_detail,
};
use crate::server::session::WsSession;
use crate::server::shadow_scope;
use anyhow::anyhow;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

pub(super) fn clear_local_unbound_runtime_binding(state: &Arc<AppState>, session: &mut WsSession) {
    if session.active_branch.is_none()
        && session.active_repo.is_none()
        && session.active_repo_id.is_none()
        && session.has_runtime_scope_binding()
    {
        clear_runtime_binding_and_revoke(state, session);
    }
}

pub(super) fn precheck_remote_unbound_scope(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope_nonce: Option<u64>,
) -> bool {
    let Some(branch) = session.active_branch.as_ref().cloned() else {
        return false;
    };
    if session.active_repo.is_some() || session.active_repo_id.is_some() {
        return false;
    }
    if let Err(error) = shadow_scope::ensure_remote_branch_available(state, &branch) {
        let error = if error.is_remote_branch_unavailable() {
            state.revoke_source_control_write_grant_for_session(session);
            shadow_scope::clear_stale_remote_branch(session);
            ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, error.detail())
        } else {
            clear_runtime_binding_and_revoke(state, session);
            ServerError::from(error)
        };
        ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
        return true;
    }
    if session.has_runtime_scope_binding() {
        clear_runtime_binding_and_revoke(state, session);
        ch.send_protocol_error_with_scope_nonce(
            ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                stale_unbound_remote_scope_detail(&branch),
            ),
            scope_nonce,
        );
        return true;
    }
    false
}

fn clear_runtime_binding_and_revoke(state: &Arc<AppState>, session: &mut WsSession) {
    state.revoke_source_control_write_grant_for_session(session);
    session.clear_active_db();
    session.clear_sync_binding();
}

pub(super) fn send_listing_error(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    let detail = detail.into();
    ch.send_protocol_error_with_scope_nonce(
        ServerError::with_detail(classify_listing_error(&detail), detail),
        scope_nonce,
    );
}

pub(super) fn map_listing_repo_scope_error(error: anyhow::Error) -> ServerError {
    if let Some(error) = RepoScopeFailure::from_anyhow(&error)
        && matches!(
            error,
            RepoScopeFailure::RemoteBranchUnavailable { .. } | RepoScopeFailure::StaleScope { .. }
        )
    {
        return ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, error.detail());
    }
    map_repo_scope_error(error)
}

pub(super) fn browser_scope_nonce(session: Option<&WsSession>) -> Option<u64> {
    session
        .filter(|session| session.is_browser_session())
        .map(WsSession::scope_nonce)
}

fn classify_listing_error(detail: &str) -> ServerErrorCode {
    map_repo_scope_error(anyhow!(detail.to_string())).code
}

#[cfg(test)]
mod tests;

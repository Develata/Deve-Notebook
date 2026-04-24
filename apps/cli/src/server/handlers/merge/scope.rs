//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Merge scope resolution helpers.

use crate::server::channel::DualChannel;
use crate::server::repo_scope::{
    map_repo_scope_error, resolve_local_counterpart_repo, resolve_session_repo_or_bootstrap_local,
};
use crate::server::{AppState, session::WsSession};
use deve_core::models::RepoId;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

use super::errors;

pub(super) fn resolve_read_repo_id(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope_nonce: Option<u64>,
) -> Option<RepoId> {
    let scope = resolve_merge_scope(state, ch, session, scope_nonce)?;
    if scope.branch.is_none() {
        return Some(scope.repo_id);
    }
    match resolve_local_counterpart_repo(state, &scope) {
        Ok(Some(local_scope)) => Some(local_scope.repo_id),
        Ok(None) => {
            errors::storage_not_found(
                ch,
                "No local repository matched the active remote branch",
                scope_nonce,
            );
            None
        }
        Err(err) => {
            ch.send_protocol_error_with_scope_nonce(map_repo_scope_error(err), scope_nonce);
            None
        }
    }
}

pub(super) fn resolve_write_repo_id(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope_nonce: Option<u64>,
) -> Option<RepoId> {
    let scope = resolve_merge_scope(state, ch, session, scope_nonce)?;
    if scope.branch.is_some() {
        ch.send_protocol_error_with_scope_nonce(
            ServerError::new(ServerErrorCode::ScRemoteBranchReadonly),
            scope_nonce,
        );
        return None;
    }
    Some(scope.repo_id)
}

pub(super) fn resolve_merge_scope(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope_nonce: Option<u64>,
) -> Option<crate::server::repo_scope::ResolvedRepo> {
    let resolved = resolve_session_repo_or_bootstrap_local(state, session);
    match resolved {
        Ok(scope) => {
            if scope.branch.is_none()
                && (session.active_repo.as_deref() != Some(scope.repo_name.as_str())
                    || session.active_repo_id != Some(scope.repo_id))
            {
                session.switch_repo(scope.repo_name.clone(), Some(scope.repo_id));
            }
            Some(scope)
        }
        Err(err) => {
            ch.send_protocol_error_with_scope_nonce(map_repo_scope_error(err), scope_nonce);
            None
        }
    }
}

#[cfg(test)]
#[path = "scope_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "scope_test.rs"]
mod tests;

#[cfg(test)]
#[path = "scope_stale_test.rs"]
mod stale_tests;

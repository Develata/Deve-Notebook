//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! Browser sync hello scope validation.

use super::SyncHelloScopeFailure;
use crate::server::session::WsSession;
use deve_core::models::RepoId;

pub(super) fn validate_browser_scope(
    session: &WsSession,
    repo_id: RepoId,
    scope_nonce: u64,
) -> Result<(), SyncHelloScopeFailure> {
    if session.active_branch.is_some() || session.active_repo_id != Some(repo_id) {
        return Err(SyncHelloScopeFailure::repo_context_invalid(
            format!(
                "Browser SyncHello scope mismatch: active_branch={:?}, active_repo_id={:?}, requested_repo_id={}",
                session.active_branch, session.active_repo_id, repo_id
            ),
            false,
        ));
    }
    if browser_runtime_binding_mismatch(session, repo_id) {
        return Err(SyncHelloScopeFailure::repo_context_invalid(
            format!(
                "Browser SyncHello runtime binding mismatch: active_repo={:?}, bound_repo_id={:?}, writer_repo_id={:?}",
                session.active_repo,
                session.bound_repo_id,
                session
                    .writer_identity
                    .as_ref()
                    .map(|writer| writer.repo_id)
            ),
            false,
        ));
    }
    if session.scope_nonce() != scope_nonce {
        return Err(SyncHelloScopeFailure::stale_scope(
            format!(
                "Browser SyncHello stale scope nonce: current_scope_nonce={}, requested_scope_nonce={}",
                session.scope_nonce(),
                scope_nonce
            ),
            false,
        ));
    }
    Ok(())
}

fn browser_runtime_binding_mismatch(session: &WsSession, repo_id: RepoId) -> bool {
    let active_db_mismatch = match session.active_repo.as_deref() {
        Some(repo_name) => {
            session.get_active_db().is_some()
                && session
                    .active_db_for(None, repo_name, Some(repo_id))
                    .is_none()
        }
        None => session.get_active_db().is_some(),
    };
    let bound_repo_mismatch = session
        .bound_repo_id
        .is_some_and(|bound_repo_id| bound_repo_id != repo_id);
    let writer_mismatch = session
        .writer_identity
        .as_ref()
        .is_some_and(|writer| writer.repo_id != repo_id);
    active_db_mismatch || bound_repo_mismatch || writer_mismatch
}

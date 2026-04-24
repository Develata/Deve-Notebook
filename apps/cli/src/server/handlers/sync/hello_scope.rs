//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Sync hello scope validation.

use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{ServerError, ServerErrorCode};

#[path = "hello_scope_browser.rs"]
mod browser;

pub(super) struct SyncHelloScopeFailure {
    pub(super) error: ServerError,
    pub(super) clear_active_repo: bool,
}

impl SyncHelloScopeFailure {
    fn repo_context_invalid(detail: String, clear_active_repo: bool) -> Self {
        Self {
            error: ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, detail),
            clear_active_repo,
        }
    }
}

pub(super) fn validate_scope(
    session: &WsSession,
    peer_id: &PeerId,
    repo_id: RepoId,
    scope_nonce: u64,
) -> Result<(), SyncHelloScopeFailure> {
    if !session.is_browser_session() {
        return validate_non_browser_scope(session, peer_id, repo_id, scope_nonce);
    }
    browser::validate_browser_scope(session, repo_id, scope_nonce)
}

fn validate_non_browser_scope(
    session: &WsSession,
    peer_id: &PeerId,
    repo_id: RepoId,
    scope_nonce: u64,
) -> Result<(), SyncHelloScopeFailure> {
    if let Some(active_branch) = session.active_branch.as_ref()
        && active_branch != peer_id
    {
        return Err(SyncHelloScopeFailure::repo_context_invalid(
            format!(
                "SyncHello peer mismatch: active_branch={}, requested_peer_id={}",
                active_branch, peer_id
            ),
            true,
        ));
    }
    if let Some(authenticated_peer_id) = session.authenticated_peer_id.as_ref()
        && authenticated_peer_id != peer_id
    {
        return Err(SyncHelloScopeFailure::repo_context_invalid(
            format!(
                "SyncHello peer mismatch: authenticated_peer_id={}, requested_peer_id={}",
                authenticated_peer_id, peer_id
            ),
            true,
        ));
    }
    if let Some(bound_repo_id) = session.bound_repo_id
        && bound_repo_id != repo_id
    {
        return Err(SyncHelloScopeFailure::repo_context_invalid(
            format!(
                "SyncHello repo mismatch: bound_repo_id={}, requested_repo_id={}",
                bound_repo_id, repo_id
            ),
            true,
        ));
    }
    if let Some(active_repo) = session.active_repo.as_deref()
        && session.active_repo_id.is_none()
    {
        return Err(SyncHelloScopeFailure::repo_context_invalid(
            format!(
                "SyncHello repo selector not resolved: active_repo={}, requested_repo_id={}",
                active_repo, repo_id
            ),
            true,
        ));
    }
    if let Some(active_repo_id) = session.active_repo_id
        && active_repo_id != repo_id
    {
        return Err(SyncHelloScopeFailure::repo_context_invalid(
            format!(
                "SyncHello repo mismatch: active_repo_id={}, requested_repo_id={}",
                active_repo_id, repo_id
            ),
            true,
        ));
    }
    if let Some(current_sync_scope_nonce) = session.sync_scope_nonce()
        && current_sync_scope_nonce != scope_nonce
    {
        return Err(SyncHelloScopeFailure::repo_context_invalid(
            format!(
                "SyncHello stale scope nonce: current_sync_scope_nonce={}, requested_scope_nonce={}",
                current_sync_scope_nonce, scope_nonce
            ),
            false,
        ));
    }
    Ok(())
}

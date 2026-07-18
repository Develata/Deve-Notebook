//! plan_ref:
//!   - 07_network#remote-import-wire-contract
//!
//! Exact local-writer capability shared by non-Source-Control product intents.

use crate::server::repo_scope::ResolvedRepo;
use crate::server::{AppState, session::WsSession};
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

pub(crate) fn require_exact_local_writer(
    state: &Arc<AppState>,
    session: &WsSession,
    scope: &ResolvedRepo,
) -> Result<(), ServerError> {
    crate::server::repo_scope::ensure_resolved_local_repo_writable(state, scope)?;
    require_exact_local_writer_identity(session, scope)
}

pub(crate) fn require_exact_local_writer_identity(
    session: &WsSession,
    scope: &ResolvedRepo,
) -> Result<(), ServerError> {
    if scope.branch.is_some() {
        return Err(ServerError::with_detail(
            ServerErrorCode::ScRemoteBranchReadonly,
            scope.repo_name.clone(),
        ));
    }
    let nonce = session.scope_nonce();
    let writer = session.writer_peer_id_for(&scope.repo_id, Some(nonce));
    if !session.is_repo_bound(&scope.repo_id)
        || !session.has_accepted_sync_hello()
        || session.sync_scope_nonce() != Some(nonce)
        || session.is_readonly()
        || writer.as_ref() != session.authenticated_peer_id.as_ref()
    {
        return Err(ServerError::new(ServerErrorCode::SyncPeerUnauthenticated));
    }
    Ok(())
}

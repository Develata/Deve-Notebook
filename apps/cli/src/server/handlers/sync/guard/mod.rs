//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! Sync message scope and peer guards.

use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{ServerError, ServerErrorCode};

use super::cleanup::{clear_remote_unbound_state, clear_stale_browser_sync_scope};

pub(super) fn require_current_sync_scope(
    ch: &DualChannel,
    session: &mut WsSession,
) -> Option<Option<u64>> {
    if !session.is_browser_session() {
        return Some(None);
    }
    match validate_browser_sync_scope_state(session) {
        Ok(scope_nonce) => Some(Some(scope_nonce)),
        Err(error) => {
            ch.send_protocol_error_with_scope_nonce(
                error.into_request_error(),
                Some(session.scope_nonce()),
            );
            None
        }
    }
}

pub(super) fn require_bound_peer(
    ch: &DualChannel,
    session: &mut WsSession,
    repo_id: RepoId,
    scope_nonce: Option<u64>,
) -> Option<PeerId> {
    if !session.is_repo_bound(&repo_id) {
        let code = if session.bound_repo_id.is_none()
            || session
                .get_active_db()
                .is_some_and(|db| db.repo_id == Some(repo_id))
        {
            ServerErrorCode::SyncRepoUnbound
        } else {
            ServerErrorCode::SyncRepoRouteMismatch
        };
        clear_remote_unbound_state(session);
        tracing::warn!(
            "Sync repo mismatch: session bound to {:?}, got {}",
            session.bound_repo_id,
            repo_id
        );
        ch.send_protocol_error_with_scope_nonce(ServerError::new(code), scope_nonce);
        return None;
    }

    let Some(peer_id) = session.authenticated_peer_id.clone() else {
        clear_remote_unbound_state(session);
        tracing::error!("Sync message without authenticated peer");
        ch.send_protocol_error_with_scope_nonce(
            ServerError::new(ServerErrorCode::SyncPeerUnauthenticated),
            scope_nonce,
        );
        return None;
    };
    Some(peer_id)
}

pub(super) fn require_delivery_scope_nonce(
    ch: &DualChannel,
    session: &mut WsSession,
    scope_nonce: Option<u64>,
) -> Option<u64> {
    if let Some(scope_nonce) = scope_nonce {
        return Some(scope_nonce);
    }
    match session.sync_scope_nonce() {
        Some(scope_nonce) => Some(scope_nonce),
        None => {
            clear_remote_unbound_state(session);
            ch.send_protocol_error_with_scope_nonce(
                ServerError::with_detail(
                    ServerErrorCode::ScRepoContextInvalid,
                    "sync scope nonce not bound",
                ),
                scope_nonce.or(session
                    .is_browser_session()
                    .then_some(session.scope_nonce())),
            );
            None
        }
    }
}

#[cfg(test)]
fn validate_browser_sync_scope(session: &mut WsSession) -> Result<u64, ServerError> {
    validate_browser_sync_scope_state(session).map_err(BrowserSyncScopeFailure::into_request_error)
}

enum BrowserSyncScopeFailure {
    NotBound,
    StaleNonce,
}

impl BrowserSyncScopeFailure {
    fn into_request_error(self) -> ServerError {
        match self {
            Self::NotBound => ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                "browser sync scope not bound",
            ),
            Self::StaleNonce => ServerError::with_detail(
                ServerErrorCode::ScStaleScope,
                "browser sync scope nonce is stale",
            ),
        }
    }
}

fn validate_browser_sync_scope_state(
    session: &mut WsSession,
) -> Result<u64, BrowserSyncScopeFailure> {
    let scope_nonce = session.scope_nonce();
    let Some(sync_scope_nonce) = session.sync_scope_nonce() else {
        clear_stale_browser_sync_scope(session);
        return Err(BrowserSyncScopeFailure::NotBound);
    };
    if sync_scope_nonce != scope_nonce {
        clear_stale_browser_sync_scope(session);
        return Err(BrowserSyncScopeFailure::StaleNonce);
    }
    Ok(sync_scope_nonce)
}

#[cfg(test)]
mod tests;

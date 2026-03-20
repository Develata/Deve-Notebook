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
    match validate_browser_sync_scope(session) {
        Ok(scope_nonce) => Some(Some(scope_nonce)),
        Err(error) => {
            ch.send_protocol_error_with_scope_nonce(error, Some(session.scope_nonce()));
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
        clear_remote_unbound_state(session);
        tracing::warn!(
            "Sync repo mismatch: session bound to {:?}, got {}",
            session.bound_repo_id,
            repo_id
        );
        ch.send_protocol_error_with_scope_nonce(
            ServerError::new(ServerErrorCode::SyncRepoUnbound),
            scope_nonce,
        );
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
                scope_nonce.or(session.is_browser_session().then_some(session.scope_nonce())),
            );
            None
        }
    }
}

fn validate_browser_sync_scope(session: &mut WsSession) -> Result<u64, ServerError> {
    let scope_nonce = session.scope_nonce();
    let Some(sync_scope_nonce) = session.sync_scope_nonce() else {
        clear_stale_browser_sync_scope(session);
        return Err(ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            "browser sync scope not bound",
        ));
    };
    if sync_scope_nonce != scope_nonce {
        clear_stale_browser_sync_scope(session);
        return Err(ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            "browser sync scope nonce is stale",
        ));
    }
    Ok(sync_scope_nonce)
}

#[cfg(test)]
mod tests {
    use super::validate_browser_sync_scope;
    use crate::server::session::WsSession;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn rejects_missing_browser_sync_scope() {
        let mut session = WsSession::new();
        session.mark_browser_session();
        session.set_scope_nonce(Some(9));

        let error = validate_browser_sync_scope(&mut session).expect_err("must fail");
        assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
        assert!(session.get_active_db().is_none());
        assert!(session.bound_repo_id.is_none());
        assert!(session.authenticated_peer_id.is_none());
        assert_eq!(session.sync_scope_nonce(), None);
    }

    #[test]
    fn rejects_stale_browser_sync_scope() {
        let mut session = WsSession::new();
        session.mark_browser_session();
        session.set_scope_nonce(Some(11));
        session.set_sync_scope_nonce(7);

        let error = validate_browser_sync_scope(&mut session).expect_err("must fail");
        assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
        assert!(session.get_active_db().is_none());
        assert!(session.bound_repo_id.is_none());
        assert!(session.authenticated_peer_id.is_none());
        assert_eq!(session.sync_scope_nonce(), None);
    }
}

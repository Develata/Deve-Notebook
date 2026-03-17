use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{ServerError, ServerErrorCode};

use super::cleanup::clear_remote_unbound_state;

pub(super) fn require_bound_peer(
    ch: &DualChannel,
    session: &mut WsSession,
    repo_id: RepoId,
) -> Option<PeerId> {
    let scope_nonce = session.is_browser_session().then(|| {
        session
            .sync_scope_nonce()
            .unwrap_or_else(|| session.scope_nonce())
    });
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

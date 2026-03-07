use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};

pub(super) fn require_bound_peer(
    ch: &DualChannel,
    session: &WsSession,
    repo_id: RepoId,
) -> Option<PeerId> {
    if !session.is_repo_bound(&repo_id) {
        tracing::warn!(
            "Sync repo mismatch: session bound to {:?}, got {}",
            session.bound_repo_id,
            repo_id
        );
        ch.send_error("Repository not bound to session".to_string());
        return None;
    }

    let Some(peer_id) = session.authenticated_peer_id.clone() else {
        tracing::error!("Sync message without authenticated peer");
        ch.send_error("Not authenticated".to_string());
        return None;
    };
    Some(peer_id)
}

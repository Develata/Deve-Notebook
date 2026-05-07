//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Sync writer readiness registration.

use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};

use super::cleanup::{clear_remote_unbound_state, clear_stale_browser_sync_scope};

pub(super) fn handle(
    ch: &DualChannel,
    session: &mut WsSession,
    repo_id: RepoId,
    peer_id: PeerId,
    scope_nonce: u64,
) {
    match validate(session, repo_id, &peer_id, scope_nonce) {
        Ok(()) => {
            session.set_writer_identity(repo_id, peer_id.clone(), scope_nonce);
            ch.unicast(ServerMessage::WriteReady {
                peer_id,
                repo_id,
                scope_nonce,
                branch: session.active_branch.clone(),
            });
        }
        Err(error) => ch.send_protocol_error_with_scope_nonce(
            error,
            session.is_browser_session().then_some(scope_nonce),
        ),
    }
}

fn validate(
    session: &mut WsSession,
    repo_id: RepoId,
    peer_id: &PeerId,
    scope_nonce: u64,
) -> Result<(), ServerError> {
    if session.is_browser_session() {
        if session.active_branch.is_some() {
            clear_stale_browser_sync_scope(session);
            return Err(ServerError::new(ServerErrorCode::ScRemoteBranchReadonly));
        }
        if session.active_repo_id != Some(repo_id) {
            clear_stale_browser_sync_scope(session);
            return Err(ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                "writer scope does not match active repo",
            ));
        }
        if browser_active_db_mismatch(session, repo_id) {
            clear_stale_browser_sync_scope(session);
            return Err(ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                "writer active db does not match active repo",
            ));
        }
        if session.scope_nonce() != scope_nonce || session.sync_scope_nonce() != Some(scope_nonce) {
            clear_stale_browser_sync_scope(session);
            return Err(ServerError::with_detail(
                ServerErrorCode::ScStaleScope,
                "writer scope nonce is stale",
            ));
        }
    }
    if session.is_readonly() {
        return Err(ServerError::new(ServerErrorCode::ScRemoteBranchReadonly));
    }
    if !session.is_repo_bound(&repo_id) {
        clear_remote_unbound_state(session);
        return Err(ServerError::new(ServerErrorCode::SyncRepoUnbound));
    }
    let Some(auth_peer_id) = session.authenticated_peer_id.as_ref() else {
        clear_remote_unbound_state(session);
        return Err(ServerError::new(ServerErrorCode::SyncPeerUnauthenticated));
    };
    if auth_peer_id != peer_id {
        clear_remote_unbound_state(session);
        return Err(ServerError::with_detail(
            ServerErrorCode::SyncPeerUnauthenticated,
            "writer peer mismatch",
        ));
    }
    Ok(())
}

fn browser_active_db_mismatch(session: &WsSession, repo_id: RepoId) -> bool {
    let Some(repo_name) = session.active_repo.as_deref() else {
        return session.get_active_db().is_some();
    };
    session.get_active_db().is_some()
        && session
            .active_db_for(None, repo_name, Some(repo_id))
            .is_none()
}

#[cfg(test)]
#[path = "writer_test.rs"]
mod tests;

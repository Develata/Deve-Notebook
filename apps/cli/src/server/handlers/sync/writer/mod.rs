//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 09_web_thin_client_ledger#write-readiness
//!
//! Sync writer readiness registration.

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{
    ensure_resolved_local_repo_writable, map_repo_scope_error, resolve_session_repo,
};
use crate::server::session::WsSession;
use crate::server::source_control_grants::SourceControlGrantBranch;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;

use super::cleanup::{clear_remote_unbound_state, clear_stale_browser_sync_scope};

pub(super) fn handle(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    repo_id: RepoId,
    peer_id: PeerId,
    scope_nonce: u64,
) {
    match validate(session, repo_id, &peer_id, scope_nonce) {
        Ok(()) => match validate_local_projection_writable(state, session, repo_id) {
            Ok(()) => {
                if let Some(auth_session_id) = session.auth_session_id().cloned()
                    && let Err(error) = state.source_control_write_grants().grant(
                        auth_session_id,
                        repo_id,
                        SourceControlGrantBranch::from_active_branch(
                            session.active_branch.as_ref(),
                        ),
                        peer_id.clone(),
                        scope_nonce,
                    )
                {
                    ch.send_protocol_error_with_scope_nonce(
                        error,
                        session.is_browser_session().then_some(scope_nonce),
                    );
                    return;
                }
                session.set_writer_identity(repo_id, peer_id.clone(), scope_nonce);
                ch.unicast(ServerMessage::WriteReady {
                    peer_id,
                    repo_id,
                    scope_nonce: scope_nonce.into(),
                    branch: session.active_branch.clone(),
                });
            }
            Err(error) => {
                state.revoke_source_control_write_grant_for_session(session);
                ch.send_protocol_error_with_scope_nonce(
                    error,
                    session.is_browser_session().then_some(scope_nonce),
                );
            }
        },
        Err(error) => {
            if let Some(auth_session_id) = session.auth_session_id() {
                state
                    .source_control_write_grants()
                    .revoke_session(auth_session_id);
            }
            ch.send_protocol_error_with_scope_nonce(
                error,
                session.is_browser_session().then_some(scope_nonce),
            );
        }
    }
}

fn validate(
    session: &mut WsSession,
    repo_id: RepoId,
    peer_id: &PeerId,
    scope_nonce: u64,
) -> Result<(), ServerError> {
    if !session.is_browser_session() {
        return Err(ServerError::with_detail(
            ServerErrorCode::SyncPeerUnauthenticated,
            "writer registration requires browser session",
        ));
    }
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

fn validate_local_projection_writable(
    state: &Arc<AppState>,
    session: &WsSession,
    repo_id: RepoId,
) -> Result<(), ServerError> {
    if session.active_branch.is_some() {
        return Ok(());
    }
    if session.active_repo_id != Some(repo_id) {
        return Ok(());
    }
    let resolved = resolve_session_repo(state, session).map_err(map_repo_scope_error)?;
    if resolved.repo_id != repo_id || resolved.branch.is_some() {
        return Err(ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            "writer scope does not match resolved local repo",
        ));
    }
    ensure_resolved_local_repo_writable(state, &resolved)?;
    state
        .repo_mutation_gate()
        .admit_mounted_repo(repo_id)
        .map(|_| ())
        .map_err(|error| error.server_error())
}

#[cfg(test)]
mod tests;

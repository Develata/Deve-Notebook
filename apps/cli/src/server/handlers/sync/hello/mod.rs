//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! Sync hello handshake boundary.

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::listing;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId, VersionVector};
use deve_core::protocol::SessionProof;
use std::sync::Arc;

mod outbound;
mod response;
mod scope;

use self::scope::validate_scope;
use super::cleanup::clear_sync_hello_scope_failure;
use super::engine;
use super::errors;

pub struct SyncHelloInput {
    pub peer_id: PeerId,
    pub peer_pubkey: Vec<u8>,
    pub session_proof: SessionProof,
    pub remote_vector: VersionVector,
    pub repo_id: RepoId,
    pub scope_nonce: u64,
}

pub(super) async fn handle(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    hello: SyncHelloInput,
) {
    let SyncHelloInput {
        peer_id,
        peer_pubkey,
        session_proof,
        remote_vector,
        repo_id,
        scope_nonce,
    } = hello;
    tracing::info!("Handling SyncHello from {} for repo {}", peer_id, repo_id);
    let scope = session.is_browser_session().then_some(scope_nonce);

    if let Err(failure) = validate_scope(session, &peer_id, repo_id, scope_nonce) {
        clear_sync_hello_scope_failure(session, failure.clear_active_repo);
        ch.send_protocol_error_with_scope_nonce(failure.error, scope);
        return;
    }

    if !session.is_browser_session() && session.has_accepted_sync_hello() {
        tracing::warn!(
            "Rejected duplicate SyncHello from {} for repo {} scope {}",
            peer_id,
            repo_id,
            scope_nonce
        );
        errors::sync_invalid_payload(
            ch,
            format!(
                "duplicate SyncHello: session already accepted peer {} repo {} scope_nonce {}",
                peer_id, repo_id, scope_nonce
            ),
            scope,
        );
        return;
    }

    let Some(handshake) = engine::with_strict_mut(state, ch, repo_id, scope, |engine| {
        let local_vector = engine.version_vector().clone();
        engine
            .handshake(
                repo_id,
                peer_id.clone(),
                &peer_pubkey,
                session_proof.signature(),
                remote_vector,
            )
            .map(|result| (local_vector, engine.clone_for_transport(), result))
    }) else {
        clear_sync_hello_scope_failure(session, false);
        return;
    };
    let (local_vector, outbound_engine, mut result) = match handshake {
        Ok(result) => result,
        Err(e) => {
            clear_sync_hello_scope_failure(session, false);
            tracing::error!("Handshake failed with {}: {}", peer_id, e);
            errors::handshake_failed(ch, e, scope);
            return;
        }
    };

    if !session.is_browser_session()
        && let Err(err) = state.repo.ensure_shadow_repo_binding(&peer_id, repo_id)
    {
        tracing::error!(
            "Failed to align shadow repo metadata for peer {} repo {}: {:?}",
            peer_id,
            repo_id,
            err
        );
        clear_sync_hello_scope_failure(session, false);
        errors::storage_persist_failed(
            ch,
            format!(
                "Failed to bind shadow repo {} for peer {}: {}",
                repo_id, peer_id, err
            ),
            scope,
        );
        return;
    }

    filter_static_fullpeer_source_sets(state, session, &peer_id, &mut result);
    session.set_authenticated(peer_id.clone());
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(scope_nonce);
    session.set_requested_sync_sources(
        result
            .to_request
            .iter()
            .map(|req| req.peer_id.clone())
            .chain(
                result
                    .snapshot_requests
                    .iter()
                    .map(|req| req.peer_id.clone()),
            ),
    );
    session.set_offered_sync_sources(result.to_send.iter().map(|req| req.peer_id.clone()));
    session.mark_sync_hello_accepted();
    tracing::info!("Session bound to peer {} and repo {}", peer_id, repo_id);

    match response::send(state, ch, repo_id, scope_nonce, local_vector) {
        Ok(()) => {}
        Err(err) => {
            clear_sync_hello_scope_failure(session, false);
            errors::request_failed(ch, format!("Failed to encode local vector: {}", err), scope);
            return;
        }
    };
    if session.is_browser_session() {
        listing::handle_list_shadows(state, ch, Some(session), None).await;
        return;
    }

    outbound::send(
        outbound::OutboundSyncContext {
            ch,
            state,
            session,
            engine: &outbound_engine,
            repo_id,
            scope,
            scope_nonce,
        },
        result,
    );
}

fn filter_static_fullpeer_source_sets(
    state: &Arc<AppState>,
    session: &WsSession,
    authenticated_peer: &PeerId,
    result: &mut deve_core::sync::protocol::HandshakeResult,
) {
    if session.is_browser_session() {
        return;
    }
    let local_peer = state.identity_key.peer_id();
    result
        .to_send
        .retain(|request| request.peer_id == local_peer);
    result
        .to_request
        .retain(|request| &request.peer_id == authenticated_peer);
    result
        .snapshot_requests
        .retain(|request| &request.peer_id == authenticated_peer);
}

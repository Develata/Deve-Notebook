use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::listing;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId, VersionVector};
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;

use super::engine;
use super::errors;

pub struct SyncHelloInput {
    pub peer_id: PeerId,
    pub pub_key: Vec<u8>,
    pub signature: Vec<u8>,
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
        pub_key,
        signature,
        remote_vector,
        repo_id,
        scope_nonce,
    } = hello;
    tracing::info!("Handling SyncHello from {} for repo {}", peer_id, repo_id);

    if let Err(error) = validate_scope(session, repo_id, scope_nonce) {
        clear_stale_browser_sync_scope(session);
        ch.send_protocol_error(error);
        return;
    }

    let Some(mut engine) = engine::load_strict(state, ch, repo_id) else {
        return;
    };
    let local_peer_id = engine.local_peer_id.clone();
    let local_vector = engine.version_vector().clone();
    let result = match engine.handshake(
        repo_id,
        peer_id.clone(),
        &pub_key,
        &signature,
        remote_vector,
    ) {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("Handshake failed with {}: {}", peer_id, e);
            errors::classified_failure(ch, format!("Handshake failed: {}", e));
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
        errors::storage_persist_failed(
            ch,
            format!(
                "Failed to bind shadow repo {} for peer {}: {}",
                repo_id, peer_id, err
            ),
        );
        return;
    }

    session.set_authenticated(peer_id.clone());
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(scope_nonce);
    tracing::info!("Session bound to peer {} and repo {}", peer_id, repo_id);

    let vec_bytes = match serde_json::to_vec(&local_vector) {
        Ok(bytes) => bytes,
        Err(err) => {
            errors::request_failed(ch, format!("Failed to encode local vector: {}", err));
            return;
        }
    };
    let mut msg = Vec::new();
    msg.extend_from_slice(b"deve-handshake");
    msg.extend_from_slice(local_peer_id.as_str().as_bytes());
    msg.extend_from_slice(&vec_bytes);

    let my_sig = state.identity_key.sign(&msg);
    ch.unicast(ServerMessage::SyncHello {
        peer_id: local_peer_id,
        repo_id,
        scope_nonce,
        pub_key: state.identity_key.public_key_bytes().to_vec(),
        signature: my_sig,
        vector: local_vector,
    });

    if session.is_browser_session() {
        listing::handle_list_shadows(state, ch, Some(session), None).await;
        return;
    }

    if !result.to_request.is_empty() {
        let requests = result
            .to_request
            .into_iter()
            .map(|req| (req.peer_id, req.range))
            .collect();
        ch.unicast(ServerMessage::SyncRequest {
            repo_id,
            branch: session.active_branch.clone(),
            requests,
        });
    }

    for req in result.snapshot_requests {
        ch.unicast(ServerMessage::SyncSnapshotRequest {
            peer_id: req.peer_id,
            repo_id: req.repo_id,
        });
    }

    let mut ops_to_push = Vec::new();
    for req in result.to_send {
        match engine.get_ops_for_sync(&req) {
            Ok(response) => ops_to_push.extend(response.ops),
            Err(err) => {
                errors::classified_failure(
                    ch,
                    format!("Failed to build sync payload for repo {}: {}", repo_id, err),
                );
                return;
            }
        }
    }

    if !ops_to_push.is_empty() {
        ch.unicast(ServerMessage::SyncPush {
            repo_id,
            scope_nonce,
            branch: session.active_branch.clone(),
            ops: ops_to_push,
        });
    }
}

fn validate_scope(
    session: &WsSession,
    repo_id: RepoId,
    scope_nonce: u64,
) -> Result<(), ServerError> {
    if !session.is_browser_session() {
        return Ok(());
    }
    if session.active_branch.is_some() || session.active_repo_id != Some(repo_id) {
        return Err(ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            format!(
                "Browser SyncHello scope mismatch: active_branch={:?}, active_repo_id={:?}, requested_repo_id={}",
                session.active_branch, session.active_repo_id, repo_id
            ),
        ));
    }
    if session.scope_nonce() != scope_nonce {
        return Err(ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            format!(
                "Browser SyncHello stale scope nonce: current_scope_nonce={}, requested_scope_nonce={}",
                session.scope_nonce(),
                scope_nonce
            ),
        ));
    }
    Ok(())
}

fn clear_stale_browser_sync_scope(session: &mut WsSession) {
    if !session.is_browser_session() {
        return;
    }
    session.clear_active_db();
    session.clear_sync_binding();
}

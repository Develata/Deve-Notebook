use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId, VersionVector};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

use super::errors;

pub struct SyncHelloInput {
    pub peer_id: PeerId,
    pub pub_key: Vec<u8>,
    pub signature: Vec<u8>,
    pub remote_vector: VersionVector,
    pub repo_id: RepoId,
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
    } = hello;
    tracing::info!("Handling SyncHello from {} for repo {}", peer_id, repo_id);

    let mut engine = match state.sync_engine.get_or_create(repo_id) {
        Some(e) => e,
        None => {
            errors::engine_unavailable(ch);
            return;
        }
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
            errors::request_failed(ch, format!("Handshake failed: {}", e));
            return;
        }
    };

    if let Err(err) = state.repo.ensure_shadow_repo_binding(&peer_id, repo_id) {
        tracing::warn!(
            "Failed to align shadow repo metadata for peer {} repo {}: {:?}",
            peer_id,
            repo_id,
            err
        );
    }

    session.set_authenticated(peer_id.clone());
    session.bind_repo(repo_id);
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
        pub_key: state.identity_key.public_key_bytes().to_vec(),
        signature: my_sig,
        vector: local_vector,
    });

    if !result.to_request.is_empty() {
        let requests = result
            .to_request
            .into_iter()
            .map(|req| (req.peer_id, req.range))
            .collect();
        ch.unicast(ServerMessage::SyncRequest { requests });
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
                errors::request_failed(
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
            ops: ops_to_push,
        });
    }
}

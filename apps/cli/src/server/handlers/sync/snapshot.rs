use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ServerMessage;
use deve_core::security::EncryptedOp;
use std::sync::Arc;

use super::errors;
use super::guard::require_bound_peer;

pub(super) async fn handle_request(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    _peer_id: PeerId,
    repo_id: RepoId,
) {
    let Some(source_peer) = require_bound_peer(ch, session, repo_id) else {
        return;
    };

    let engine = match state.sync_engine.get_or_create(repo_id) {
        Some(e) => e,
        None => {
            errors::engine_unavailable(ch);
            return;
        }
    };
    tracing::info!("Handling SnapshotRequest from {}", source_peer);

    let request = deve_core::sync::protocol::SyncSnapshotRequest {
        peer_id: source_peer.clone(),
        repo_id,
    };

    match engine.get_snapshot_for_sync(&request) {
        Ok(response) => {
            tracing::info!(
                "Sending snapshot with {} ops to {}",
                response.ops.len(),
                source_peer
            );
            ch.unicast(ServerMessage::SyncPushSnapshot {
                peer_id: engine.local_peer_id.clone(),
                repo_id: response.repo_id,
                ops: response.ops,
            });
        }
        Err(e) => {
            tracing::error!("Failed to generate snapshot for {}: {:?}", source_peer, e);
            errors::request_failed(ch, format!("Failed to generate snapshot: {}", e));
        }
    }
}

pub(super) async fn handle_push(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    _peer_id: PeerId,
    repo_id: RepoId,
    ops: Vec<EncryptedOp>,
) {
    let Some(source_peer) = require_bound_peer(ch, session, repo_id) else {
        return;
    };

    let mut engine = match state.sync_engine.get_or_create(repo_id) {
        Some(e) => e,
        None => {
            errors::engine_unavailable(ch);
            return;
        }
    };
    tracing::info!(
        "Handling PushSnapshot from {} ({} ops)",
        source_peer,
        ops.len()
    );

    let response = deve_core::sync::protocol::SyncResponse {
        peer_id: source_peer.clone(),
        repo_id,
        ops,
    };

    match engine.apply_remote_snapshot(response) {
        Ok(seq) => tracing::info!(
            "Applied snapshot from {}. Updated VV to seq {}",
            source_peer,
            seq
        ),
        Err(e) => {
            tracing::error!("Failed to apply snapshot from {}: {:?}", source_peer, e);
            errors::sync_apply_failed(ch, format!("Failed to apply snapshot: {}", e));
        }
    }
}

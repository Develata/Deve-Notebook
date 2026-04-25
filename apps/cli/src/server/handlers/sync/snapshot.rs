use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ServerMessage;
use deve_core::security::EncryptedOp;
use std::sync::Arc;

use super::engine;
use super::errors;
use super::guard::{require_bound_peer, require_current_sync_scope, require_delivery_scope_nonce};

pub(super) async fn handle_request(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    _peer_id: PeerId,
    repo_id: RepoId,
) {
    let Some(scope) = require_current_sync_scope(ch, session) else {
        return;
    };
    let Some(source_peer) = require_bound_peer(ch, session, repo_id, scope) else {
        return;
    };

    tracing::info!("Handling SnapshotRequest from {}", source_peer);

    let request = deve_core::sync::protocol::SyncSnapshotRequest {
        peer_id: source_peer.clone(),
        repo_id,
    };

    let Some(snapshot) = engine::with_strict(state, ch, repo_id, scope, |engine| {
        engine
            .get_snapshot_for_sync(&request)
            .map(|response| (engine.local_peer_id.clone(), response))
    }) else {
        return;
    };

    match snapshot {
        Ok((local_peer_id, response)) => {
            let Some(delivery_scope_nonce) = require_delivery_scope_nonce(ch, session, scope)
            else {
                return;
            };
            tracing::info!(
                "Sending snapshot with {} ops to {}",
                response.ops.len(),
                source_peer
            );
            ch.unicast(ServerMessage::SyncPushSnapshot {
                peer_id: local_peer_id,
                repo_id: response.repo_id,
                scope_nonce: delivery_scope_nonce,
                branch: session.active_branch.clone(),
                ops: response.ops,
            });
        }
        Err(e) => {
            tracing::error!("Failed to generate snapshot for {}: {:?}", source_peer, e);
            errors::classified_failure(ch, format!("Failed to generate snapshot: {}", e), scope);
        }
    }
}

pub(super) async fn handle_push(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    _peer_id: PeerId,
    repo_id: RepoId,
    ops: Vec<EncryptedOp>,
) {
    let Some(scope) = require_current_sync_scope(ch, session) else {
        return;
    };
    let Some(source_peer) = require_bound_peer(ch, session, repo_id, scope) else {
        return;
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

    let Some(applied) = engine::with_strict_mut(state, ch, repo_id, scope, |engine| {
        engine.receive_remote_snapshot(response)
    }) else {
        return;
    };

    match applied {
        Ok(count) => tracing::info!("Handled snapshot from {} with {} ops", source_peer, count),
        Err(e) => {
            tracing::error!("Failed to apply snapshot from {}: {:?}", source_peer, e);
            errors::sync_apply_failed(ch, format!("Failed to apply snapshot: {}", e), scope);
        }
    }
}

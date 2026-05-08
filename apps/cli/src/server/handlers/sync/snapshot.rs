//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

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
    peer_id: PeerId,
    repo_id: RepoId,
    reason: Option<String>,
) {
    let Some(scope) = require_current_sync_scope(ch, session) else {
        return;
    };
    let Some(transport_peer) = require_bound_peer(ch, session, repo_id, scope) else {
        return;
    };
    if !session.allows_sync_export_source(&peer_id) {
        errors::sync_peer_unauthenticated(
            ch,
            format!(
                "SyncSnapshotRequest source {} was not offered to transport {}",
                peer_id, transport_peer
            ),
            scope,
        );
        return;
    }

    tracing::info!(
        "Handling SnapshotRequest for source {} from transport {}",
        peer_id,
        transport_peer
    );

    let request = deve_core::sync::protocol::SyncSnapshotRequest {
        peer_id: peer_id.clone(),
        repo_id,
        reason: reason.or_else(|| Some("explicit-sync-snapshot-request".to_string())),
    };

    let Some(snapshot) = engine::with_strict(state, ch, repo_id, scope, |engine| {
        let server_vector = engine.version_vector().clone();
        engine
            .get_snapshot_for_sync(&request)
            .map(|response| (server_vector, response))
    }) else {
        return;
    };

    match snapshot {
        Ok((server_vector, response)) => {
            let Some(delivery_scope_nonce) = require_delivery_scope_nonce(ch, session, scope)
            else {
                return;
            };
            tracing::info!(
                "Sending snapshot with {} ops for source {}",
                response.ops.len(),
                response.peer_id
            );
            ch.unicast(ServerMessage::SyncPushSnapshot {
                source_peer_id: response.peer_id,
                repo_id: response.repo_id,
                scope_nonce: delivery_scope_nonce.into(),
                branch: session.active_branch.clone(),
                server_vector,
                snapshot_kind: Some("full".to_string()),
                payload: response.ops,
            });
        }
        Err(e) => {
            tracing::error!("Failed to generate snapshot for {}: {:?}", peer_id, e);
            errors::snapshot_generation_failed(
                ch,
                format!("Failed to generate snapshot: {}", e),
                scope,
            );
        }
    }
}

pub(super) async fn handle_push(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    peer_id: PeerId,
    repo_id: RepoId,
    payload: Vec<EncryptedOp>,
) {
    let Some(scope) = require_current_sync_scope(ch, session) else {
        return;
    };
    let Some(transport_peer) = require_bound_peer(ch, session, repo_id, scope) else {
        return;
    };
    if !session.allows_sync_source(&peer_id) {
        errors::sync_peer_unauthenticated(
            ch,
            format!(
                "SyncPushSnapshot source {} was not requested from transport {}",
                peer_id, transport_peer
            ),
            scope,
        );
        return;
    }

    tracing::info!(
        "Handling PushSnapshot source {} via transport {} ({} ops)",
        peer_id,
        transport_peer,
        payload.len()
    );

    let response = deve_core::sync::protocol::SyncResponse {
        peer_id: peer_id.clone(),
        repo_id,
        ops: payload,
    };

    let Some(applied) = engine::with_strict_mut(state, ch, repo_id, scope, |engine| {
        engine.receive_remote_snapshot(response)
    }) else {
        return;
    };

    match applied {
        Ok(count) => tracing::info!("Handled snapshot from {} with {} ops", peer_id, count),
        Err(e) => {
            tracing::error!("Failed to apply snapshot from {}: {:?}", peer_id, e);
            errors::sync_apply_failed(ch, format!("Failed to apply snapshot: {}", e), scope);
        }
    }
}

//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 05_network#relay-proxy-attribution-contract
//!   - 06_repository#repo-scope-runtime

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId, VersionVector};
use deve_core::protocol::{
    RelayProxySnapshotRouteInput, ServerMessage, SyncPayloadKind, SyncSourceProof,
    plan_relay_proxy_snapshot_route,
};
use deve_core::security::EncryptedOp;
use std::sync::Arc;

use super::errors;
use super::guard::{require_bound_peer, require_current_sync_scope, require_delivery_scope_nonce};
use super::{SyncPushSnapshotInput, engine};

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
            let source_peer_id = response.peer_id.clone();
            ch.unicast(ServerMessage::SyncPushSnapshot {
                source_peer_id,
                repo_id: response.repo_id,
                scope_nonce: delivery_scope_nonce.into(),
                branch: session.active_branch.clone(),
                server_vector: server_vector.clone(),
                snapshot_kind: Some("full".to_string()),
                source_proof: snapshot_source_proof(
                    state,
                    response.repo_id,
                    &response.peer_id,
                    &server_vector,
                    &response.ops,
                ),
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
    input: SyncPushSnapshotInput,
) {
    let SyncPushSnapshotInput {
        peer_id,
        repo_id,
        server_vector,
        source_proof,
        ops: payload,
    } = input;
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
    let route = match plan_relay_proxy_snapshot_route(RelayProxySnapshotRouteInput {
        expected_repo_id: repo_id,
        authenticated_transport_peer: transport_peer.clone(),
        declared_source_peer: peer_id.clone(),
        target_peer: state.identity_key.peer_id(),
        source_proof_present: source_proof.is_some(),
    }) {
        Ok(route) => route,
        Err(err) => {
            errors::sync_invalid_payload(
                ch,
                format!("invalid sync snapshot relay/proxy route: {}", err),
                scope,
            );
            return;
        }
    };
    if let Err(err) = validate_snapshot_source_proof(
        repo_id,
        &peer_id,
        &server_vector,
        source_proof.as_ref(),
        &payload,
        route.indirect_transport,
    ) {
        errors::sync_invalid_payload(
            ch,
            format!("invalid sync snapshot source proof: {}", err),
            scope,
        );
        return;
    }

    tracing::info!(
        "Handling PushSnapshot source {} via transport {} ({} ops)",
        route.source_peer,
        route.transport_peer,
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

fn snapshot_source_proof(
    state: &Arc<AppState>,
    repo_id: RepoId,
    peer_id: &PeerId,
    server_vector: &VersionVector,
    payload: &[EncryptedOp],
) -> Option<SyncSourceProof> {
    if peer_id != &state.identity_key.peer_id() {
        return None;
    }
    match SyncSourceProof::sign(
        repo_id,
        peer_id,
        server_vector,
        SyncPayloadKind::Snapshot,
        payload,
        &state.identity_key,
    ) {
        Ok(proof) => Some(proof),
        Err(err) => {
            tracing::warn!("Failed to sign local sync snapshot source proof: {}", err);
            None
        }
    }
}

fn validate_snapshot_source_proof(
    repo_id: RepoId,
    peer_id: &PeerId,
    server_vector: &VersionVector,
    source_proof: Option<&SyncSourceProof>,
    payload: &[EncryptedOp],
    required: bool,
) -> Result<(), deve_core::protocol::SyncSourceProofError> {
    match source_proof {
        Some(proof) => proof.verify(
            repo_id,
            peer_id,
            server_vector,
            SyncPayloadKind::Snapshot,
            payload,
        ),
        None if required => Err(deve_core::protocol::SyncSourceProofError::Missing),
        None => Ok(()),
    }
}

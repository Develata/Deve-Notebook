//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 07_network#relay-proxy-attribution-contract
//!   - 04_repository#repo-scope-runtime

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{
    ServerMessage, SourceProofRequirement, SyncPushAttributionInput, SyncPushHeader,
    validate_sync_push_attribution,
};
use deve_core::security::EncryptedOp;
use deve_core::sync::protocol as sync_proto;
use std::sync::Arc;

use super::engine;
use super::errors;
use super::guard::{require_bound_peer, require_current_sync_scope, require_delivery_scope_nonce};

pub(super) async fn handle_request(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    repo_id: RepoId,
    requests: Vec<(PeerId, (u64, u64))>,
) {
    let Some(scope) = require_current_sync_scope(ch, session) else {
        return;
    };
    if require_bound_peer(ch, session, repo_id, scope).is_none() {
        return;
    }
    if let Some(peer_id) = requests
        .iter()
        .map(|(peer_id, _range)| peer_id)
        .find(|peer_id| !session.allows_sync_export_source(peer_id))
    {
        errors::sync_peer_unauthenticated(
            ch,
            format!(
                "SyncRequest source {} was not offered in this scope",
                peer_id
            ),
            scope,
        );
        return;
    }

    let Some(responses) = engine::with_strict(state, ch, repo_id, scope, |engine| {
        let header_vector = engine.version_vector().clone();
        let mut responses = Vec::new();
        for (peer_id, range) in requests {
            let sync_req = sync_proto::SyncRequest {
                peer_id,
                repo_id,
                range,
            };
            match engine.get_ops_for_sync(&sync_req) {
                Ok(response) => responses.push(response),
                Err(err) => return Err(err),
            }
        }
        Ok((header_vector, responses))
    }) else {
        return;
    };
    let (header_vector, responses) = match responses {
        Ok(result) => result,
        Err(err) => {
            errors::sync_payload_build_failed(
                ch,
                format!(
                    "Failed to build sync response for repo {}: {}",
                    repo_id, err
                ),
                scope,
            );
            return;
        }
    };

    let non_empty: Vec<_> = responses
        .into_iter()
        .filter(|response| !response.ops.is_empty())
        .collect();
    if non_empty.is_empty() {
        return;
    }

    let Some(delivery_scope_nonce) = require_delivery_scope_nonce(ch, session, scope) else {
        return;
    };
    for response in non_empty {
        let header = SyncPushHeader::diff(
            response.repo_id,
            response.peer_id.clone(),
            header_vector.clone(),
        );
        let header = attach_local_source_proof(state, header, &response.ops);
        ch.unicast(ServerMessage::SyncPush {
            source_peer_id: response.peer_id,
            repo_id: response.repo_id,
            header,
            scope_nonce: delivery_scope_nonce.into(),
            branch: session.active_branch.clone(),
            encrypted_payload: response.ops,
        });
    }
}

pub(super) fn attach_local_source_proof(
    state: &Arc<AppState>,
    mut header: SyncPushHeader,
    payload: &[EncryptedOp],
) -> SyncPushHeader {
    if header.peer_id == state.identity_key.peer_id()
        && let Err(err) = header.sign_source(payload, &state.identity_key)
    {
        tracing::warn!("Failed to sign local sync source proof: {}", err);
    }
    header
}

pub(super) async fn handle_push(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    peer_id: PeerId,
    repo_id: RepoId,
    header: SyncPushHeader,
    encrypted_payload: Vec<EncryptedOp>,
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
                "SyncPush source {} was not requested from transport {}",
                peer_id, transport_peer
            ),
            scope,
        );
        return;
    }
    let target_peer = state.identity_key.peer_id();
    let route = match validate_sync_push_attribution(SyncPushAttributionInput {
        expected_repo_id: repo_id,
        authenticated_transport_peer: &transport_peer,
        declared_source_peer: &peer_id,
        target_peer: &target_peer,
        header: &header,
        payload: &encrypted_payload,
        source_proof_requirement: SourceProofRequirement::IndirectOnly,
    }) {
        Ok(route) => route,
        Err(err) => {
            errors::sync_invalid_payload(
                ch,
                format!("invalid sync source attribution: {}", err),
                scope,
            );
            return;
        }
    };
    tracing::debug!(
        "Handling SyncPush source {} via transport {}",
        route.source_peer,
        route.transport_peer
    );

    let response = sync_proto::SyncResponse {
        peer_id: peer_id.clone(),
        repo_id,
        ops: encrypted_payload,
    };

    let Some(applied) = engine::with_strict_mut(state, ch, repo_id, scope, |engine| {
        engine.receive_remote_ops(response)
    }) else {
        return;
    };

    match applied {
        Ok(count) => tracing::info!(
            "Handled {} remote ops from {} for repo {}",
            count,
            peer_id,
            repo_id
        ),
        Err(e) => {
            tracing::error!("Failed to apply ops for repo {}: {:?}", repo_id, e);
            errors::sync_apply_failed(
                ch,
                format!("Failed to apply sync ops for repo {}: {}", repo_id, e),
                scope,
            );
        }
    }
}

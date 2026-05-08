//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{ServerMessage, SyncPayloadKind, SyncPushHeader};
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
            errors::classified_failure(
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
    if !sync_push_header_matches(&header, repo_id, &peer_id) {
        errors::sync_apply_failed(
            ch,
            format!(
                "invalid sync payload header: route repo/source {}/{} but header repo/source/kind {}/{}/{}",
                repo_id,
                peer_id,
                header.repo_id,
                header.peer_id,
                sync_payload_kind_name(&header.payload_kind)
            ),
            scope,
        );
        return;
    }
    tracing::debug!(
        "Handling SyncPush source {} via transport {}",
        peer_id,
        transport_peer
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

fn sync_push_header_matches(header: &SyncPushHeader, repo_id: RepoId, peer_id: &PeerId) -> bool {
    header.repo_id == repo_id
        && &header.peer_id == peer_id
        && header.payload_kind == SyncPayloadKind::Diff
}

fn sync_payload_kind_name(kind: &SyncPayloadKind) -> &'static str {
    match kind {
        SyncPayloadKind::Diff => "diff",
        SyncPayloadKind::Snapshot => "snapshot",
    }
}

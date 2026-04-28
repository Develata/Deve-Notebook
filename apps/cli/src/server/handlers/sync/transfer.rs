//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ServerMessage;
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
        Ok(responses)
    }) else {
        return;
    };
    let responses = match responses {
        Ok(responses) => responses,
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
        ch.unicast(ServerMessage::SyncPush {
            peer_id: response.peer_id,
            repo_id: response.repo_id,
            scope_nonce: delivery_scope_nonce,
            branch: session.active_branch.clone(),
            ops: response.ops,
        });
    }
}

pub(super) async fn handle_push(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    peer_id: PeerId,
    repo_id: RepoId,
    ops: Vec<EncryptedOp>,
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
    tracing::debug!(
        "Handling SyncPush source {} via transport {}",
        peer_id,
        transport_peer
    );

    let response = sync_proto::SyncResponse {
        peer_id: peer_id.clone(),
        repo_id,
        ops,
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

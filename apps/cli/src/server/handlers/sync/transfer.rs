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

    let Some(responses) = engine::with_strict(state, ch, repo_id, scope, |engine| {
        let mut ops_to_push = Vec::new();
        for (peer_id, range) in requests {
            let sync_req = sync_proto::SyncRequest {
                peer_id,
                repo_id,
                range,
            };
            match engine.get_ops_for_sync(&sync_req) {
                Ok(response) => ops_to_push.extend(response.ops),
                Err(err) => return Err(err),
            }
        }
        Ok(ops_to_push)
    }) else {
        return;
    };
    let ops_to_push = match responses {
        Ok(ops) => ops,
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

    if !ops_to_push.is_empty() {
        let Some(delivery_scope_nonce) = require_delivery_scope_nonce(ch, session, scope) else {
            return;
        };
        ch.unicast(ServerMessage::SyncPush {
            repo_id,
            scope_nonce: delivery_scope_nonce,
            branch: session.active_branch.clone(),
            ops: ops_to_push,
        });
    }
}

pub(super) async fn handle_push(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    repo_id: RepoId,
    ops: Vec<EncryptedOp>,
) {
    let Some(scope) = require_current_sync_scope(ch, session) else {
        return;
    };
    let Some(source_peer) = require_bound_peer(ch, session, repo_id, scope) else {
        return;
    };

    let response = sync_proto::SyncResponse {
        peer_id: source_peer,
        repo_id,
        ops,
    };

    let Some(applied) = engine::with_strict_mut(state, ch, repo_id, scope, |engine| {
        engine.receive_remote_ops(response)
    }) else {
        return;
    };

    match applied {
        Ok(count) => tracing::info!("Handled {} remote ops for repo {}", count, repo_id),
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

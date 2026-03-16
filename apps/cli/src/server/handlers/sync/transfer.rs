use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ServerMessage;
use deve_core::security::EncryptedOp;
use deve_core::sync::protocol as sync_proto;
use std::sync::Arc;

use super::errors;
use super::engine;
use super::guard::require_bound_peer;

pub(super) async fn handle_request(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    repo_id: RepoId,
    requests: Vec<(PeerId, (u64, u64))>,
) {
    if !session.is_repo_bound(&repo_id) {
        super::cleanup::clear_remote_unbound_state(session);
        tracing::warn!(
            "SyncRequest repo mismatch: session bound to {:?}, got {}",
            session.bound_repo_id,
            repo_id
        );
        ch.send_sync_repo_unbound();
        return;
    }

    let Some(engine) = engine::load_strict(state, ch, repo_id) else {
        return;
    };
    let mut ops_to_push = Vec::new();

    for (peer_id, range) in requests {
        let sync_req = sync_proto::SyncRequest {
            peer_id,
            repo_id,
            range,
        };
        match engine.get_ops_for_sync(&sync_req) {
            Ok(response) => ops_to_push.extend(response.ops),
            Err(err) => {
                errors::classified_failure(
                    ch,
                    format!(
                        "Failed to build sync response for repo {}: {}",
                        repo_id, err
                    ),
                );
                return;
            }
        }
    }

    if !ops_to_push.is_empty() {
        ch.unicast(ServerMessage::SyncPush {
            repo_id,
            scope_nonce: session
                .sync_scope_nonce()
                .unwrap_or_else(|| session.scope_nonce()),
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
    let Some(source_peer) = require_bound_peer(ch, session, repo_id) else {
        return;
    };

    let Some(mut engine) = engine::load_strict(state, ch, repo_id) else {
        return;
    };

    let response = sync_proto::SyncResponse {
        peer_id: source_peer,
        repo_id,
        ops,
    };

    match engine.apply_remote_ops(response) {
        Ok(count) => tracing::info!("Applied {} ops for repo {}", count, repo_id),
        Err(e) => {
            tracing::error!("Failed to apply ops for repo {}: {:?}", repo_id, e);
            errors::sync_apply_failed(
                ch,
                format!("Failed to apply sync ops for repo {}: {}", repo_id, e),
            );
        }
    }
}

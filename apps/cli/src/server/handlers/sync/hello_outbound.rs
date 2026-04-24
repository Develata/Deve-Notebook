//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Sync hello outbound follow-up messages.

use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::RepoId;
use deve_core::protocol::ServerMessage;
use deve_core::sync::engine::SyncEngine;
use deve_core::sync::protocol::HandshakeResult;

use super::super::cleanup::clear_sync_hello_scope_failure;
use super::super::errors;

pub(super) fn send(
    ch: &DualChannel,
    session: &mut WsSession,
    engine: &SyncEngine,
    result: HandshakeResult,
    repo_id: RepoId,
    scope: Option<u64>,
    scope_nonce: u64,
) {
    send_requests(ch, session, repo_id, result.to_request);
    send_snapshot_requests(ch, result.snapshot_requests);
    send_pushes(
        ch,
        session,
        engine,
        repo_id,
        scope,
        scope_nonce,
        result.to_send,
    );
}

fn send_requests(
    ch: &DualChannel,
    session: &WsSession,
    repo_id: RepoId,
    requests: Vec<deve_core::sync::protocol::SyncRequest>,
) {
    if requests.is_empty() {
        return;
    }
    ch.unicast(ServerMessage::SyncRequest {
        repo_id,
        branch: session.active_branch.clone(),
        requests: requests
            .into_iter()
            .map(|req| (req.peer_id, req.range))
            .collect(),
    });
}

fn send_snapshot_requests(
    ch: &DualChannel,
    requests: Vec<deve_core::sync::protocol::SyncSnapshotRequest>,
) {
    for req in requests {
        ch.unicast(ServerMessage::SyncSnapshotRequest {
            peer_id: req.peer_id,
            repo_id: req.repo_id,
        });
    }
}

fn send_pushes(
    ch: &DualChannel,
    session: &mut WsSession,
    engine: &SyncEngine,
    repo_id: RepoId,
    scope: Option<u64>,
    scope_nonce: u64,
    requests: Vec<deve_core::sync::protocol::SyncRequest>,
) {
    let mut ops_to_push = Vec::new();
    for req in requests {
        match engine.get_ops_for_sync(&req) {
            Ok(response) => ops_to_push.extend(response.ops),
            Err(err) => {
                clear_sync_hello_scope_failure(session, false);
                errors::classified_failure(
                    ch,
                    format!("Failed to build sync payload for repo {}: {}", repo_id, err),
                    scope,
                );
                return;
            }
        }
    }

    if !ops_to_push.is_empty() {
        ch.unicast(ServerMessage::SyncPush {
            repo_id,
            scope_nonce,
            branch: session.active_branch.clone(),
            ops: ops_to_push,
        });
    }
}

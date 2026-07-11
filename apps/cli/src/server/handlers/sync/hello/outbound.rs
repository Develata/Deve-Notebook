//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! Sync hello outbound follow-up messages.

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::RepoId;
use deve_core::protocol::{ServerMessage, SyncPushHeader};
use deve_core::sync::engine::SyncEngine;
use deve_core::sync::protocol::HandshakeResult;

use super::super::cleanup::clear_sync_hello_scope_failure;
use super::super::errors;

pub(super) struct OutboundSyncContext<'a> {
    pub ch: &'a DualChannel,
    pub state: &'a std::sync::Arc<AppState>,
    pub session: &'a mut WsSession,
    pub engine: &'a SyncEngine,
    pub repo_id: RepoId,
    pub scope: Option<u64>,
    pub scope_nonce: u64,
}

pub(super) fn send(ctx: OutboundSyncContext<'_>, result: HandshakeResult) {
    let known_vector = ctx.engine.version_vector().clone();
    send_requests(
        ctx.ch,
        ctx.session,
        ctx.repo_id,
        known_vector.clone(),
        result.to_request,
    );
    send_snapshot_requests(ctx.ch, known_vector, result.snapshot_requests);
    send_pushes(ctx, result.to_send);
}

fn send_requests(
    ch: &DualChannel,
    session: &WsSession,
    repo_id: RepoId,
    known_vector: deve_core::models::VersionVector,
    requests: Vec<deve_core::sync::protocol::SyncRequest>,
) {
    if requests.is_empty() {
        return;
    }
    ch.unicast(ServerMessage::SyncRequest {
        repo_id,
        branch: session.active_branch.clone(),
        known_vector,
        requests: requests
            .into_iter()
            .map(|req| (req.peer_id, req.range))
            .collect(),
    });
}

fn send_snapshot_requests(
    ch: &DualChannel,
    known_vector: deve_core::models::VersionVector,
    requests: Vec<deve_core::sync::protocol::SyncSnapshotRequest>,
) {
    for req in requests {
        ch.unicast(ServerMessage::SyncSnapshotRequest {
            source_peer_id: req.peer_id,
            repo_id: req.repo_id,
            known_vector: known_vector.clone(),
            reason: req.reason,
        });
    }
}

fn send_pushes(
    ctx: OutboundSyncContext<'_>,
    requests: Vec<deve_core::sync::protocol::SyncRequest>,
) {
    for req in requests {
        match ctx.engine.get_ops_for_sync(&req) {
            Ok(mut response) => {
                if response.ops.is_empty() {
                    continue;
                }
                if ctx.session.authenticated_peer_id.is_some()
                    && let Err(err) = crate::server::p2p::fault_injection::maybe_inject_sequence_gap(
                        ctx.state,
                        "full_peer_sync_hello_push",
                        response.range,
                        &mut response.ops,
                    )
                {
                    clear_sync_hello_scope_failure(ctx.session, false);
                    errors::sync_payload_build_failed(
                        ctx.ch,
                        format!(
                            "Failed to inject armed P2P sequence-gap test fault for repo {}: {}",
                            ctx.repo_id, err
                        ),
                        ctx.scope,
                    );
                    return;
                }
                let header = SyncPushHeader::diff(
                    response.repo_id,
                    response.peer_id.clone(),
                    ctx.engine.version_vector().clone(),
                );
                let header = match super::super::transfer::attach_local_source_proof(
                    ctx.state,
                    header,
                    &response.ops,
                ) {
                    Ok(header) => header,
                    Err(err) => {
                        clear_sync_hello_scope_failure(ctx.session, false);
                        errors::sync_payload_build_failed(
                            ctx.ch,
                            format!(
                                "Failed to sign local sync source proof for repo {}: {}",
                                ctx.repo_id, err
                            ),
                            ctx.scope,
                        );
                        return;
                    }
                };
                let (range_start, range_end) = response
                    .range
                    .expect("incremental sync response must carry its closed range");
                ctx.ch.unicast(ServerMessage::SyncPush {
                    source_peer_id: response.peer_id,
                    repo_id: response.repo_id,
                    range_start,
                    range_end,
                    header,
                    scope_nonce: ctx.scope_nonce.into(),
                    branch: ctx.session.active_branch.clone(),
                    encrypted_payload: response.ops,
                });
            }
            Err(err) => {
                clear_sync_hello_scope_failure(ctx.session, false);
                errors::sync_payload_build_failed(
                    ctx.ch,
                    format!(
                        "Failed to build sync payload for repo {}: {}",
                        ctx.repo_id, err
                    ),
                    ctx.scope,
                );
                return;
            }
        }
    }
}

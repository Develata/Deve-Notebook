//! plan_ref:
//!   - 07_network#full-peer-mesh-v1

use crate::server::AppState;
use crate::server::p2p::stats::ExchangeStats;
use anyhow::{Context, Result, anyhow};
use deve_core::models::{PeerId, RepoId, VersionVector};
use deve_core::protocol::frame::encode_client_binary;
use deve_core::protocol::{ClientMessage, SyncPayloadKind, SyncPushHeader, SyncSourceProof};
use deve_core::security::EncryptedOp;
use deve_core::sync::protocol as sync_proto;
use futures::SinkExt;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;

pub(super) async fn send_requested_ops<S>(
    state: &Arc<AppState>,
    socket: &mut S,
    repo_id: RepoId,
    requests: Vec<(PeerId, (u64, u64))>,
    stats: &mut ExchangeStats,
) -> Result<()>
where
    S: futures::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let (header_vector, responses) = state
        .sync_engine
        .with_strict_engine(repo_id, |engine| {
            let header_vector = engine.version_vector().clone();
            let responses = requests
                .into_iter()
                .map(|(peer_id, range)| {
                    engine.get_ops_for_sync(&sync_proto::SyncRequest {
                        peer_id,
                        repo_id,
                        range,
                    })
                })
                .collect::<Vec<_>>();
            (header_vector, responses)
        })
        .with_context(|| format!("Failed to build P2P sync response for {repo_id}"))?;

    for response in responses {
        let response = response?;
        if response.ops.is_empty() {
            continue;
        }
        let header = signed_local_diff_header(
            state,
            response.repo_id,
            &response.peer_id,
            header_vector.clone(),
            &response.ops,
        )?;
        send_client_message(
            socket,
            ClientMessage::SyncPush {
                source_peer_id: response.peer_id,
                repo_id: response.repo_id,
                header,
                encrypted_payload: response.ops,
            },
        )
        .await?;
        stats.sent_pushes += 1;
    }
    Ok(())
}

pub(super) async fn send_requested_snapshot<S>(
    state: &Arc<AppState>,
    socket: &mut S,
    source_peer_id: PeerId,
    repo_id: RepoId,
    reason: Option<String>,
    stats: &mut ExchangeStats,
) -> Result<()>
where
    S: futures::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let request = sync_proto::SyncSnapshotRequest {
        peer_id: source_peer_id,
        repo_id,
        reason,
    };
    let (server_vector, response) = state
        .sync_engine
        .with_strict_engine(repo_id, |engine| {
            (
                engine.version_vector().clone(),
                engine.get_snapshot_for_sync(&request),
            )
        })
        .with_context(|| format!("Failed to build P2P snapshot response for {repo_id}"))?;
    let response = response?;
    let source_proof = snapshot_source_proof(
        state,
        response.repo_id,
        &response.peer_id,
        &server_vector,
        &response.ops,
    )?;
    send_client_message(
        socket,
        ClientMessage::SyncPushSnapshot {
            source_peer_id: response.peer_id,
            repo_id: response.repo_id,
            server_vector,
            snapshot_kind: Some("full".to_string()),
            source_proof: Some(source_proof),
            payload: response.ops,
        },
    )
    .await?;
    stats.sent_snapshots += 1;
    Ok(())
}

fn send_sync_response(
    peer_id: PeerId,
    repo_id: RepoId,
    ops: Vec<EncryptedOp>,
) -> sync_proto::SyncResponse {
    sync_proto::SyncResponse {
        peer_id,
        repo_id,
        ops,
    }
}

pub(super) fn receive_remote_ops(
    state: &Arc<AppState>,
    peer_id: PeerId,
    repo_id: RepoId,
    ops: Vec<EncryptedOp>,
) -> Result<u64> {
    state
        .sync_engine
        .with_strict_engine_mut(repo_id, |engine| {
            engine.receive_remote_ops(send_sync_response(peer_id, repo_id, ops))
        })
        .with_context(|| format!("Failed to apply P2P sync ops for {repo_id}"))?
}

pub(super) fn receive_remote_snapshot(
    state: &Arc<AppState>,
    peer_id: PeerId,
    repo_id: RepoId,
    ops: Vec<EncryptedOp>,
) -> Result<u64> {
    state
        .sync_engine
        .with_strict_engine_mut(repo_id, |engine| {
            engine.receive_remote_snapshot(send_sync_response(peer_id, repo_id, ops))
        })
        .with_context(|| format!("Failed to apply P2P sync snapshot for {repo_id}"))?
}

async fn send_client_message<S>(socket: &mut S, message: ClientMessage) -> Result<()>
where
    S: futures::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let encoded = encode_client_binary(&message).context("Failed to encode P2P client frame")?;
    socket
        .send(Message::Binary(encoded))
        .await
        .context("Failed to send P2P client frame")
}

fn signed_local_diff_header(
    state: &Arc<AppState>,
    repo_id: RepoId,
    peer_id: &PeerId,
    vector: VersionVector,
    payload: &[EncryptedOp],
) -> Result<SyncPushHeader> {
    if peer_id != &state.identity_key.peer_id() {
        return Err(anyhow!(
            "P2P cannot sign non-local diff source {} for repo {}",
            peer_id,
            repo_id
        ));
    }
    let mut header = SyncPushHeader::diff(repo_id, peer_id.clone(), vector);
    header
        .sign_source(payload, &state.identity_key)
        .context("Failed to sign P2P diff source proof")?;
    Ok(header)
}

fn snapshot_source_proof(
    state: &Arc<AppState>,
    repo_id: RepoId,
    peer_id: &PeerId,
    server_vector: &VersionVector,
    payload: &[EncryptedOp],
) -> Result<SyncSourceProof> {
    if peer_id != &state.identity_key.peer_id() {
        return Err(anyhow!(
            "P2P cannot sign non-local snapshot source {} for repo {}",
            peer_id,
            repo_id
        ));
    }
    SyncSourceProof::sign(
        repo_id,
        peer_id,
        server_vector,
        SyncPayloadKind::Snapshot,
        payload,
        &state.identity_key,
    )
    .context("Failed to sign P2P snapshot source proof")
}

//! plan_ref:
//!   - 07_network#full-peer-mesh-v1
//!   - 07_network#full-peer-ws-admission

use crate::server::AppState;
use crate::server::p2p::source_sets::sync_source_sets_for_hello;
use crate::server::p2p::stats::ExchangeStats;
use crate::server::p2p::transfer::{
    receive_remote_ops, receive_remote_snapshot, send_requested_ops, send_requested_snapshot,
};
use crate::server::p2p::transport::{decode_server_message, handle_transport_control_frame};
use crate::server::p2p::validation::{
    InboundSnapshotValidation, validate_authenticated_frame, validate_inbound_push,
    validate_inbound_snapshot, validate_requested_sources,
};
use anyhow::{Context, Result, anyhow};
use deve_core::config::P2pPeerConfig;
use deve_core::models::RepoId;
use deve_core::protocol::ServerMessage;
use deve_core::sync::handshake_proof::verify_sync_hello_proof;
use futures::StreamExt;
use std::sync::Arc;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::tungstenite::Message;

const HANDSHAKE_READ_TIMEOUT: Duration = Duration::from_secs(5);
const EXCHANGE_IDLE_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const MAX_EXCHANGE_FRAMES: usize = 64;

pub(super) async fn drive_sync_exchange<S>(
    peer: &P2pPeerConfig,
    repo_id: RepoId,
    state: Arc<AppState>,
    socket: &mut S,
) -> Result<ExchangeStats>
where
    S: futures::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + futures::Sink<Message, Error = tokio_tungstenite::tungstenite::Error>
        + Unpin,
{
    let mut stats = ExchangeStats::default();
    for frame_index in 0..MAX_EXCHANGE_FRAMES {
        let read_timeout = if stats.saw_hello {
            EXCHANGE_IDLE_TIMEOUT
        } else {
            HANDSHAKE_READ_TIMEOUT
        };
        let frame = match timeout(read_timeout, socket.next()).await {
            Ok(Some(Ok(frame))) => frame,
            Ok(Some(Err(err))) => return Err(err).context("P2P peer returned websocket error"),
            Ok(None) if stats.saw_hello => return Ok(stats),
            Ok(None) => return Err(anyhow!("P2P peer closed websocket during handshake")),
            Err(_) if stats.saw_hello => return Ok(stats),
            Err(_) => return Err(anyhow!("P2P handshake timed out")),
        };
        let Some(frame) = handle_transport_control_frame(socket, frame).await? else {
            continue;
        };
        let message = decode_server_message(frame)
            .with_context(|| format!("Failed to decode P2P server frame {frame_index}"))?;
        handle_server_message(peer, repo_id, &state, socket, message, &mut stats).await?;
    }
    if stats.saw_hello {
        Ok(stats)
    } else {
        Err(anyhow!("P2P handshake ended before SyncHello"))
    }
}

pub(super) async fn handle_server_message<S>(
    peer: &P2pPeerConfig,
    repo_id: RepoId,
    state: &Arc<AppState>,
    socket: &mut S,
    message: ServerMessage,
    stats: &mut ExchangeStats,
) -> Result<()>
where
    S: futures::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    match message {
        ServerMessage::SyncHello {
            peer_id,
            repo_id: hello_repo_id,
            pub_key,
            signature,
            vector,
            ..
        } => {
            if hello_repo_id != repo_id {
                return Err(anyhow!(
                    "P2P peer {} returned repo {} during hello, expected {}",
                    peer.label,
                    hello_repo_id,
                    repo_id
                ));
            }
            if peer_id == state.identity_key.peer_id() {
                return Err(anyhow!(
                    "P2P self-loop rejected after handshake for peer {}",
                    peer.label
                ));
            }
            if peer_id.as_str() != peer.peer_id {
                return Err(anyhow!(
                    "P2P peer {} authenticated peer_id {} did not match configured peer_id {}",
                    peer.label,
                    peer_id,
                    peer.peer_id
                ));
            }
            verify_sync_hello_proof(&peer_id, &pub_key, &signature, &vector).map_err(|err| {
                anyhow!("P2P peer {} SyncHello proof rejected: {err}", peer.label)
            })?;
            let source_sets = sync_source_sets_for_hello(state, repo_id, &peer_id, &vector)?;
            stats.allowed_export_sources = source_sets.allowed_export_sources;
            stats.requested_import_sources = source_sets.requested_import_sources;
            stats.authenticated_peer_id = Some(peer_id);
            stats.saw_hello = true;
            Ok(())
        }
        ServerMessage::SyncRequest {
            repo_id: frame_repo_id,
            requests,
            ..
        } => {
            validate_authenticated_frame(peer, repo_id, stats, frame_repo_id)?;
            validate_requested_sources(
                peer,
                frame_repo_id,
                stats,
                requests.iter().map(|(peer_id, _)| peer_id),
            )?;
            send_requested_ops(state, socket, frame_repo_id, requests, stats).await?;
            Ok(())
        }
        ServerMessage::SyncSnapshotRequest {
            source_peer_id,
            repo_id: frame_repo_id,
            reason,
            ..
        } => {
            validate_authenticated_frame(peer, repo_id, stats, frame_repo_id)?;
            validate_requested_sources(
                peer,
                frame_repo_id,
                stats,
                std::iter::once(&source_peer_id),
            )?;
            send_requested_snapshot(state, socket, source_peer_id, frame_repo_id, reason, stats)
                .await?;
            Ok(())
        }
        ServerMessage::SyncPush {
            source_peer_id,
            repo_id: frame_repo_id,
            header,
            encrypted_payload,
            ..
        } => {
            validate_authenticated_frame(peer, repo_id, stats, frame_repo_id)?;
            validate_inbound_push(
                peer,
                frame_repo_id,
                stats,
                &state.identity_key.peer_id(),
                &source_peer_id,
                &header,
                &encrypted_payload,
            )?;
            let count =
                receive_remote_ops(state, source_peer_id, frame_repo_id, encrypted_payload)?;
            stats.applied_pushes += u64::from(count > 0);
            Ok(())
        }
        ServerMessage::SyncPushSnapshot {
            source_peer_id,
            repo_id: frame_repo_id,
            server_vector,
            source_proof,
            payload,
            ..
        } => {
            validate_authenticated_frame(peer, repo_id, stats, frame_repo_id)?;
            let target_peer = state.identity_key.peer_id();
            validate_inbound_snapshot(
                peer,
                frame_repo_id,
                stats,
                InboundSnapshotValidation {
                    target_peer: &target_peer,
                    source_peer_id: &source_peer_id,
                    server_vector: &server_vector,
                    source_proof: source_proof.as_ref(),
                    payload: &payload,
                },
            )?;
            let count = receive_remote_snapshot(state, source_peer_id, frame_repo_id, payload)?;
            stats.applied_snapshots += u64::from(count > 0);
            Ok(())
        }
        ServerMessage::ProtocolError { error, .. } => {
            Err(anyhow!("P2P peer returned protocol error: {:?}", error))
        }
        _ => Ok(()),
    }
}

//! plan_ref:
//!   - 07_network#full-peer-mesh-v1
//!   - 07_network#full-peer-ws-admission

use crate::server::AppState;
use anyhow::{Context, Result, anyhow};
use deve_core::config::{P2pConfig, P2pPeerConfig};
use deve_core::models::{PeerId, RepoId, VersionVector};
use deve_core::protocol::frame::{decode_server_binary, decode_server_json, encode_client_binary};
use deve_core::protocol::{
    ClientMessage, ScopeNonce, ServerMessage, SessionProof, SyncPayloadKind, SyncPushHeader,
    SyncSourceProof,
};
use deve_core::security::EncryptedOp;
use deve_core::security::IdentityKeyPair;
use deve_core::sync::protocol as sync_proto;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

const HANDSHAKE_READ_TIMEOUT: Duration = Duration::from_secs(5);
const EXCHANGE_IDLE_TIMEOUT: Duration = Duration::from_millis(750);
const MAX_EXCHANGE_FRAMES: usize = 64;

pub(super) fn spawn_mesh_connectors(config: P2pConfig, state: Arc<AppState>) {
    if !config.enabled {
        return;
    }

    let interval = Duration::from_millis(config.connect_interval_ms.max(1_000));
    for peer in config.peers.into_iter().filter(|peer| peer.enabled) {
        let state = state.clone();
        tokio::spawn(async move {
            loop {
                if let Err(err) = connect_peer_once(&peer, state.clone()).await {
                    tracing::warn!(
                        peer_label = %peer.label,
                        peer_id = %peer.peer_id,
                        "P2P mesh connector attempt failed: {err}"
                    );
                }
                tokio::time::sleep(interval).await;
            }
        });
    }
}

async fn connect_peer_once(peer: &P2pPeerConfig, state: Arc<AppState>) -> Result<()> {
    let token = std::env::var(&peer.auth_token_env)
        .with_context(|| format!("P2P token env is missing for peer {}", peer.label))?;
    if token.is_empty() {
        return Err(anyhow!("P2P token env is empty for peer {}", peer.label));
    }

    let mut request = peer
        .ws_url
        .as_str()
        .into_client_request()
        .with_context(|| format!("Invalid P2P ws_url for peer {}", peer.label))?;
    let auth_header = HeaderValue::from_str(&format!("Bearer {token}"))
        .with_context(|| format!("Invalid P2P bearer header for peer {}", peer.label))?;
    request.headers_mut().insert("authorization", auth_header);

    let repo_id = parse_repo_id(peer)?;
    let hello = build_sync_hello(&state, repo_id)?;
    let encoded = encode_client_binary(&hello).context("Failed to encode P2P SyncHello")?;

    let (mut socket, _) = connect_async(request)
        .await
        .with_context(|| format!("Failed to connect P2P peer {}", peer.label))?;
    socket
        .send(Message::Binary(encoded.into()))
        .await
        .with_context(|| format!("Failed to send SyncHello to P2P peer {}", peer.label))?;

    let stats = drive_sync_exchange(peer, repo_id, state, &mut socket).await?;
    tracing::info!(
        peer_label = %peer.label,
        peer_id = %peer.peer_id,
        authenticated_peer_id = stats
            .authenticated_peer_id
            .as_ref()
            .map(PeerId::as_str)
            .unwrap_or("unknown"),
        repo_id = %repo_id,
        sent_pushes = stats.sent_pushes,
        sent_snapshots = stats.sent_snapshots,
        applied_pushes = stats.applied_pushes,
        applied_snapshots = stats.applied_snapshots,
        "P2P mesh connector handshake completed"
    );
    Ok(())
}

#[derive(Default)]
struct ExchangeStats {
    saw_hello: bool,
    authenticated_peer_id: Option<PeerId>,
    sent_pushes: u64,
    sent_snapshots: u64,
    applied_pushes: u64,
    applied_snapshots: u64,
}

async fn drive_sync_exchange<S>(
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
        let message = decode_server_message(frame)
            .with_context(|| format!("Failed to decode P2P server frame {frame_index}"))?;
        handle_server_message(peer, repo_id, &state, socket, message, &mut stats).await?;
    }
    Ok(stats)
}

fn decode_server_message(frame: Message) -> Result<ServerMessage> {
    match frame {
        Message::Binary(bytes) => decode_server_binary(bytes.as_ref()).map_err(|err| anyhow!(err)),
        Message::Text(text) => decode_server_json(&text).map_err(|err| anyhow!(err)),
        Message::Ping(_) | Message::Pong(_) => Err(anyhow!("unexpected P2P control frame")),
        Message::Close(_) => Err(anyhow!("P2P peer closed websocket")),
        other => Err(anyhow!("unsupported P2P websocket frame: {other:?}")),
    }
}

async fn handle_server_message<S>(
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
            if peer_id.as_str() != peer.peer_id {
                tracing::debug!(
                    peer_label = %peer.label,
                    configured_peer_id = %peer.peer_id,
                    authenticated_peer_id = %peer_id,
                    "P2P mesh peer_id differs from static label"
                );
            }
            stats.authenticated_peer_id = Some(peer_id);
            stats.saw_hello = true;
            Ok(())
        }
        ServerMessage::SyncRequest {
            repo_id, requests, ..
        } => {
            send_requested_ops(state, socket, repo_id, requests, stats).await?;
            Ok(())
        }
        ServerMessage::SyncSnapshotRequest {
            source_peer_id,
            repo_id,
            reason,
            ..
        } => {
            send_requested_snapshot(state, socket, source_peer_id, repo_id, reason, stats).await?;
            Ok(())
        }
        ServerMessage::SyncPush {
            source_peer_id,
            repo_id,
            encrypted_payload,
            ..
        } => {
            let count = receive_remote_ops(state, source_peer_id, repo_id, encrypted_payload)?;
            stats.applied_pushes += u64::from(count > 0);
            Ok(())
        }
        ServerMessage::SyncPushSnapshot {
            source_peer_id,
            repo_id,
            payload,
            ..
        } => {
            let count = receive_remote_snapshot(state, source_peer_id, repo_id, payload)?;
            stats.applied_snapshots += u64::from(count > 0);
            Ok(())
        }
        ServerMessage::ProtocolError { error, .. } => {
            Err(anyhow!("P2P peer returned protocol error: {:?}", error))
        }
        _ => Ok(()),
    }
}

async fn send_requested_ops<S>(
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
        let mut header = SyncPushHeader::diff(
            response.repo_id,
            response.peer_id.clone(),
            header_vector.clone(),
        );
        sign_local_diff_source(state, &mut header, &response.ops);
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

async fn send_requested_snapshot<S>(
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
    );
    send_client_message(
        socket,
        ClientMessage::SyncPushSnapshot {
            source_peer_id: response.peer_id,
            repo_id: response.repo_id,
            server_vector,
            snapshot_kind: Some("full".to_string()),
            source_proof,
            payload: response.ops,
        },
    )
    .await?;
    stats.sent_snapshots += 1;
    Ok(())
}

async fn send_client_message<S>(socket: &mut S, message: ClientMessage) -> Result<()>
where
    S: futures::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let encoded = encode_client_binary(&message).context("Failed to encode P2P client frame")?;
    socket
        .send(Message::Binary(encoded.into()))
        .await
        .context("Failed to send P2P client frame")
}

fn receive_remote_ops(
    state: &Arc<AppState>,
    peer_id: PeerId,
    repo_id: RepoId,
    ops: Vec<EncryptedOp>,
) -> Result<u64> {
    state
        .sync_engine
        .with_strict_engine_mut(repo_id, |engine| {
            engine.receive_remote_ops(sync_proto::SyncResponse {
                peer_id,
                repo_id,
                ops,
            })
        })
        .with_context(|| format!("Failed to apply P2P sync ops for {repo_id}"))?
}

fn receive_remote_snapshot(
    state: &Arc<AppState>,
    peer_id: PeerId,
    repo_id: RepoId,
    ops: Vec<EncryptedOp>,
) -> Result<u64> {
    state
        .sync_engine
        .with_strict_engine_mut(repo_id, |engine| {
            engine.receive_remote_snapshot(sync_proto::SyncResponse {
                peer_id,
                repo_id,
                ops,
            })
        })
        .with_context(|| format!("Failed to apply P2P sync snapshot for {repo_id}"))?
}

fn sign_local_diff_source(
    state: &Arc<AppState>,
    header: &mut SyncPushHeader,
    payload: &[EncryptedOp],
) {
    if header.peer_id == state.identity_key.peer_id()
        && let Err(err) = header.sign_source(payload, &state.identity_key)
    {
        tracing::warn!("Failed to sign P2P diff source proof: {}", err);
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
            tracing::warn!("Failed to sign P2P snapshot source proof: {}", err);
            None
        }
    }
}

fn parse_repo_id(peer: &P2pPeerConfig) -> Result<RepoId> {
    uuid::Uuid::parse_str(&peer.repo_id)
        .with_context(|| format!("Invalid P2P repo_id for peer {}", peer.label))
}

fn build_sync_hello(state: &Arc<AppState>, repo_id: RepoId) -> Result<ClientMessage> {
    let vector = state
        .sync_engine
        .with_strict_engine(repo_id, |engine| engine.version_vector().clone())
        .with_context(|| {
            format!("Failed to refresh local vector before P2P hello for {repo_id}")
        })?;
    Ok(signed_sync_hello(
        state.identity_key.as_ref(),
        repo_id,
        vector,
    ))
}

fn signed_sync_hello(
    identity: &IdentityKeyPair,
    repo_id: RepoId,
    vector: VersionVector,
) -> ClientMessage {
    let peer_id = identity.peer_id();
    let sorted_map: std::collections::BTreeMap<_, _> = vector.iter().collect();
    let vec_bytes = serde_json::to_vec(&sorted_map).expect("version vector serializes");
    let mut msg = Vec::new();
    msg.extend_from_slice(b"deve-handshake");
    msg.extend_from_slice(peer_id.as_str().as_bytes());
    msg.extend_from_slice(&vec_bytes);

    ClientMessage::SyncHello {
        peer_id,
        peer_pubkey: identity.public_key_bytes().to_vec(),
        session_proof: SessionProof::new(identity.sign(&msg)),
        vector,
        repo_id,
        scope_nonce: ScopeNonce::new(0),
    }
}

#[cfg(test)]
mod tests {
    use super::signed_sync_hello;
    use deve_core::models::VersionVector;
    use deve_core::protocol::ClientMessage;
    use deve_core::security::IdentityKeyPair;
    use deve_core::security::keypair::verify_signature;

    #[test]
    fn p2p_mesh_sync_hello_is_signed_for_full_peer_admission_path() {
        let identity = IdentityKeyPair::generate();
        let repo_id = uuid::Uuid::new_v4();
        let hello = signed_sync_hello(&identity, repo_id, VersionVector::new());

        match hello {
            ClientMessage::SyncHello {
                peer_id,
                peer_pubkey,
                session_proof,
                vector,
                repo_id: decoded_repo,
                scope_nonce,
            } => {
                let sorted_map: std::collections::BTreeMap<_, _> = vector.iter().collect();
                let vec_bytes = serde_json::to_vec(&sorted_map).expect("vector");
                let mut msg = Vec::new();
                msg.extend_from_slice(b"deve-handshake");
                msg.extend_from_slice(peer_id.as_str().as_bytes());
                msg.extend_from_slice(&vec_bytes);

                assert_eq!(decoded_repo, repo_id);
                assert_eq!(scope_nonce.get(), 0);
                assert!(verify_signature(
                    &peer_pubkey,
                    &msg,
                    session_proof.signature()
                ));
            }
            other => panic!("expected SyncHello, got {other:?}"),
        }
    }
}

//! plan_ref:
//!   - 07_network#full-peer-mesh-v1
//!   - 07_network#full-peer-ws-admission

use crate::server::AppState;
use anyhow::{Context, Result, anyhow};
use deve_core::config::P2pPeerConfig;
use deve_core::models::{PeerId, RepoId, VersionVector};
use deve_core::protocol::frame::{decode_server_binary, decode_server_json, encode_client_binary};
use deve_core::protocol::{
    ClientMessage, DirectSyncPushAttributionInput, DirectSyncSnapshotAttributionInput, ScopeNonce,
    ServerMessage, SessionProof, SourceProofRequirement, SyncPayloadKind, SyncPushHeader,
    SyncSourceProof, validate_direct_sync_push_attribution,
    validate_direct_sync_snapshot_attribution,
};
use deve_core::security::EncryptedOp;
use deve_core::security::IdentityKeyPair;
use deve_core::sync::handshake_proof::{sign_sync_hello, verify_sync_hello_proof};
use deve_core::sync::protocol as sync_proto;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

const HANDSHAKE_READ_TIMEOUT: Duration = Duration::from_secs(5);
const EXCHANGE_IDLE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_EXCHANGE_FRAMES: usize = 64;

pub(super) use crate::server::p2p_connector::spawn_mesh_connectors;

pub(super) async fn connect_peer_once(
    peer: &P2pPeerConfig,
    state: Arc<AppState>,
) -> Result<ExchangeStats> {
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
        .send(Message::Binary(encoded))
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
    Ok(stats)
}

#[derive(Debug, Default)]
pub(super) struct ExchangeStats {
    pub(super) saw_hello: bool,
    pub(super) authenticated_peer_id: Option<PeerId>,
    pub(super) allowed_export_sources: Vec<PeerId>,
    pub(super) sent_pushes: u64,
    pub(super) sent_snapshots: u64,
    pub(super) applied_pushes: u64,
    pub(super) applied_snapshots: u64,
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
    if stats.saw_hello {
        Ok(stats)
    } else {
        Err(anyhow!("P2P handshake ended before SyncHello"))
    }
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
            stats.allowed_export_sources =
                allowed_export_sources_for_hello(state, repo_id, &vector)?;
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

fn validate_authenticated_frame(
    peer: &P2pPeerConfig,
    expected_repo_id: RepoId,
    stats: &ExchangeStats,
    frame_repo_id: RepoId,
) -> Result<()> {
    validate_authenticated_exchange(peer, stats)?;
    if frame_repo_id != expected_repo_id {
        return Err(anyhow!(
            "P2P peer {} sent repo {} after handshake for configured repo {}",
            peer.label,
            frame_repo_id,
            expected_repo_id
        ));
    }
    Ok(())
}

fn validate_authenticated_exchange<'a>(
    peer: &P2pPeerConfig,
    stats: &'a ExchangeStats,
) -> Result<&'a PeerId> {
    if !stats.saw_hello {
        return Err(anyhow!(
            "P2P peer {} sent sync payload before SyncHello",
            peer.label
        ));
    }
    stats.authenticated_peer_id.as_ref().ok_or_else(|| {
        anyhow!(
            "P2P peer {} sent sync payload before authenticated SyncHello",
            peer.label
        )
    })
}

fn validate_requested_sources<'a>(
    peer: &P2pPeerConfig,
    repo_id: RepoId,
    stats: &ExchangeStats,
    sources: impl IntoIterator<Item = &'a PeerId>,
) -> Result<()> {
    validate_authenticated_exchange(peer, stats)?;
    for source in sources {
        if !stats.allowed_export_sources.contains(source) {
            return Err(anyhow!(
                "P2P request source {} was not offered to peer {} for repo {}",
                source,
                peer.label,
                repo_id
            ));
        }
    }
    Ok(())
}

fn validate_inbound_push(
    peer: &P2pPeerConfig,
    repo_id: RepoId,
    stats: &ExchangeStats,
    target_peer: &PeerId,
    source_peer_id: &PeerId,
    header: &SyncPushHeader,
    payload: &[EncryptedOp],
) -> Result<()> {
    let authenticated_peer = validate_authenticated_exchange(peer, stats)?;
    validate_direct_sync_push_attribution(DirectSyncPushAttributionInput {
        expected_repo_id: repo_id,
        authenticated_peer,
        declared_source_peer: source_peer_id,
        target_peer,
        header,
        payload,
        source_proof_requirement: SourceProofRequirement::IndirectOnly,
    })
    .with_context(|| {
        format!(
            "P2P SyncPush source attribution rejected (source proof rejected) for peer {} repo {}",
            peer.label, repo_id
        )
    })?;
    Ok(())
}

struct InboundSnapshotValidation<'a> {
    target_peer: &'a PeerId,
    source_peer_id: &'a PeerId,
    server_vector: &'a VersionVector,
    source_proof: Option<&'a SyncSourceProof>,
    payload: &'a [EncryptedOp],
}

fn validate_inbound_snapshot(
    peer: &P2pPeerConfig,
    repo_id: RepoId,
    stats: &ExchangeStats,
    input: InboundSnapshotValidation<'_>,
) -> Result<()> {
    let authenticated_peer = validate_authenticated_exchange(peer, stats)?;
    validate_direct_sync_snapshot_attribution(DirectSyncSnapshotAttributionInput {
        expected_repo_id: repo_id,
        authenticated_peer,
        declared_source_peer: input.source_peer_id,
        target_peer: input.target_peer,
        server_vector: input.server_vector,
        source_proof: input.source_proof,
        payload: input.payload,
        source_proof_requirement: SourceProofRequirement::Always,
    })
    .with_context(|| {
        format!(
            "P2P SyncPushSnapshot source attribution rejected (source proof rejected) for peer {} repo {}",
            peer.label, repo_id
        )
    })?;
    Ok(())
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
        .send(Message::Binary(encoded))
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

fn allowed_export_sources_for_hello(
    state: &Arc<AppState>,
    repo_id: RepoId,
    remote_vector: &VersionVector,
) -> Result<Vec<PeerId>> {
    let local_peer = state.identity_key.peer_id();
    state
        .sync_engine
        .with_strict_engine(repo_id, |engine| {
            let (to_send, _, _) =
                sync_proto::compute_diff_requests(engine.version_vector(), remote_vector, repo_id);
            let mut sources = Vec::new();
            for request in to_send {
                if request.peer_id == local_peer && !sources.contains(&request.peer_id) {
                    sources.push(request.peer_id);
                }
            }
            sources
        })
        .with_context(|| format!("Failed to compute P2P offered sources for {repo_id}"))
}

fn signed_sync_hello(
    identity: &IdentityKeyPair,
    repo_id: RepoId,
    vector: VersionVector,
) -> ClientMessage {
    let peer_id = identity.peer_id();
    let signature = sign_sync_hello(identity, &vector).expect("version vector serializes");

    ClientMessage::SyncHello {
        peer_id,
        peer_pubkey: identity.public_key_bytes().to_vec(),
        session_proof: SessionProof::new(signature),
        vector,
        repo_id,
        scope_nonce: ScopeNonce::new(0),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        InboundSnapshotValidation, MAX_EXCHANGE_FRAMES, allowed_export_sources_for_hello,
        drive_sync_exchange, handle_server_message, signed_sync_hello, validate_inbound_snapshot,
    };
    use crate::server::{AppState, tree_state::RepoTreeRegistry};
    use deve_core::config::{GitBridgeMode, P2pPeerConfig, SyncMode};
    use deve_core::ledger::RepoManager;
    use deve_core::models::{DocId, LedgerEntry, Op, PeerId, VersionVector};
    use deve_core::protocol::frame::encode_server_binary;
    use deve_core::protocol::{ClientMessage, ScopeNonce, ServerMessage, SyncPushHeader};
    use deve_core::security::{EncryptedOp, IdentityKeyPair};
    use deve_core::sync::handshake_proof::{
        sign_sync_hello, sync_hello_transcript, verify_sync_hello_proof,
    };
    use deve_core::sync::{SyncManager, repo_scoped::RepoScopedSyncEngine};
    use futures::{Sink, Stream};
    use std::collections::VecDeque;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll};
    use tokio::time::{Duration, Sleep};
    use tokio_tungstenite::tungstenite::{Error as WsError, Message};

    struct MockSocket {
        incoming: VecDeque<Result<Message, WsError>>,
        sent: Vec<Message>,
    }

    enum DelayedFrame {
        Ready(Message),
        After {
            sleep: Pin<Box<Sleep>>,
            message: Option<Message>,
        },
    }

    struct DelayedSocket {
        incoming: VecDeque<DelayedFrame>,
        sent: Vec<Message>,
    }

    impl MockSocket {
        fn new(incoming: Vec<Message>) -> Self {
            Self {
                incoming: incoming.into_iter().map(Ok).collect(),
                sent: Vec::new(),
            }
        }
    }

    impl DelayedSocket {
        fn new(incoming: Vec<DelayedFrame>) -> Self {
            Self {
                incoming: incoming.into(),
                sent: Vec::new(),
            }
        }
    }

    impl Stream for MockSocket {
        type Item = Result<Message, WsError>;

        fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(self.incoming.pop_front())
        }
    }

    impl Sink<Message> for MockSocket {
        type Error = WsError;

        fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
            self.sent.push(item);
            Ok(())
        }

        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
    }

    impl Stream for DelayedSocket {
        type Item = Result<Message, WsError>;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let Some(frame) = self.incoming.front_mut() else {
                return Poll::Ready(None);
            };
            match frame {
                DelayedFrame::Ready(_) => match self.incoming.pop_front() {
                    Some(DelayedFrame::Ready(message)) => Poll::Ready(Some(Ok(message))),
                    _ => unreachable!("front frame is ready"),
                },
                DelayedFrame::After { sleep, message } => {
                    if sleep.as_mut().poll(cx).is_pending() {
                        return Poll::Pending;
                    }
                    let message = message.take().expect("delayed message");
                    self.incoming.pop_front();
                    Poll::Ready(Some(Ok(message)))
                }
            }
        }
    }

    impl Sink<Message> for DelayedSocket {
        type Error = WsError;

        fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
            self.sent.push(item);
            Ok(())
        }

        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
    }

    fn test_state(identity: Arc<IdentityKeyPair>) -> anyhow::Result<Arc<AppState>> {
        Ok(test_state_with_dir(identity)?.1)
    }

    fn test_state_with_dir(
        identity: Arc<IdentityKeyPair>,
    ) -> anyhow::Result<(tempfile::TempDir, Arc<AppState>)> {
        let dir = tempfile::tempdir()?;
        let mut repo = RepoManager::init(
            dir.path().join("ledger"),
            10,
            Some("default"),
            Some("urn:default"),
        )?;
        repo.set_projection_base_for_all_local_repos_checked(dir.path().join("vault"))?;
        let repo = Arc::new(repo);
        let (tx, _rx) = tokio::sync::broadcast::channel(8);
        let sync_manager = Arc::new(SyncManager::new_checked(repo.clone())?);
        Ok((
            dir,
            Arc::new(AppState {
                repo: repo.clone(),
                sync_manager,
                tx,
                plugins: Vec::new(),
                sync_engine: Arc::new(RepoScopedSyncEngine::new(
                    identity.peer_id(),
                    repo,
                    SyncMode::Auto,
                )),
                tree_manager: Arc::new(RepoTreeRegistry::new()),
                #[cfg(feature = "search")]
                search_available: false,
                identity_key: identity,
                git_bridge: GitBridgeMode::Mirror,
            }),
        ))
    }

    fn peer(repo_id: uuid::Uuid) -> P2pPeerConfig {
        peer_with_id(repo_id, "peer-b")
    }

    fn peer_with_id(repo_id: uuid::Uuid, peer_id: &str) -> P2pPeerConfig {
        P2pPeerConfig {
            label: "peer-b".into(),
            peer_id: peer_id.into(),
            repo_id: repo_id.to_string(),
            ws_url: "ws://127.0.0.1:3002/ws".into(),
            auth_token_env: "DEVE_TEST_TOKEN".into(),
            enabled: true,
        }
    }

    fn dummy_payload() -> Vec<EncryptedOp> {
        vec![EncryptedOp {
            doc_id: None,
            seq: 1,
            ciphertext: vec![1, 2, 3],
            nonce: vec![0; 12],
        }]
    }

    fn append_local_op(state: &Arc<AppState>, repo_id: uuid::Uuid) -> anyhow::Result<()> {
        state.sync_engine.get_or_create_strict(repo_id)?;
        let doc_id = DocId::new();
        let local_peer = state.identity_key.peer_id();
        state.repo.append_generated_op_in_local_repo(
            state.repo.local_repo_name(),
            doc_id,
            local_peer.clone(),
            |seq| {
                LedgerEntry::new_content(
                    doc_id,
                    Op::Insert {
                        pos: 0,
                        content: "local".into(),
                    },
                    1,
                    local_peer.clone(),
                    seq,
                    None,
                    None,
                )
            },
        )?;
        Ok(())
    }

    fn append_remote_shadow_op(
        state: &Arc<AppState>,
        repo_id: uuid::Uuid,
        remote_peer: &PeerId,
    ) -> anyhow::Result<()> {
        let doc_id = DocId::new();
        let entry = LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "remote-shadow".into(),
            },
            1,
            remote_peer.clone(),
            1,
            None,
            None,
        );
        state
            .repo
            .append_remote_ops(remote_peer, &repo_id, &[entry])?;
        Ok(())
    }

    fn authenticated_stats(peer_id: PeerId) -> super::ExchangeStats {
        super::ExchangeStats {
            saw_hello: true,
            authenticated_peer_id: Some(peer_id),
            ..Default::default()
        }
    }

    fn signed_server_hello(
        identity: &IdentityKeyPair,
        repo_id: uuid::Uuid,
        vector: VersionVector,
    ) -> ServerMessage {
        let peer_id = identity.peer_id();
        let signature = sign_sync_hello(identity, &vector).expect("version vector serializes");

        ServerMessage::SyncHello {
            peer_id,
            repo_id,
            scope_nonce: ScopeNonce::new(0),
            pub_key: identity.public_key_bytes().to_vec(),
            signature,
            vector,
        }
    }

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
                assert_eq!(decoded_repo, repo_id);
                assert_eq!(scope_nonce.get(), 0);
                verify_sync_hello_proof(&peer_id, &peer_pubkey, session_proof.signature(), &vector)
                    .expect("client SyncHello proof verifies");
            }
            other => panic!("expected SyncHello, got {other:?}"),
        }
    }

    #[test]
    fn p2p_fullpeer_offer_set_excludes_third_party_shadow_sources() -> anyhow::Result<()> {
        let identity = Arc::new(IdentityKeyPair::generate());
        let local_peer = identity.peer_id();
        let (_dir, state) = test_state_with_dir(identity)?;
        let repo_id = state
            .repo
            .get_repo_info_for(None, Some(state.repo.local_repo_name()))?
            .expect("repo info")
            .uuid;
        let third_party = PeerId::new("peer-a");

        append_local_op(&state, repo_id)?;
        append_remote_shadow_op(&state, repo_id, &third_party)?;

        let offered = allowed_export_sources_for_hello(&state, repo_id, &VersionVector::new())?;

        assert!(offered.contains(&local_peer));
        assert!(
            !offered.contains(&third_party),
            "FullPeer v1 must not advertise shadow sources without retained origin proof"
        );
        Ok(())
    }

    #[tokio::test]
    async fn p2p_exchange_rejects_frame_limit_without_sync_hello() -> anyhow::Result<()> {
        let identity = Arc::new(IdentityKeyPair::generate());
        let (_dir, state) = test_state_with_dir(identity)?;
        let repo_id = uuid::Uuid::new_v4();
        let pong = Message::Binary(encode_server_binary(&ServerMessage::Pong)?);
        let mut socket = MockSocket::new(vec![pong; MAX_EXCHANGE_FRAMES]);

        let err = drive_sync_exchange(&peer(repo_id), repo_id, state, &mut socket)
            .await
            .expect_err("missing SyncHello must fail");

        assert!(err.to_string().contains("before SyncHello"));
        Ok(())
    }

    #[tokio::test]
    async fn p2p_exchange_rejects_request_before_sync_hello() -> anyhow::Result<()> {
        let identity = Arc::new(IdentityKeyPair::generate());
        let state = test_state(identity)?;
        let repo_id = uuid::Uuid::new_v4();
        let message = ServerMessage::SyncRequest {
            repo_id,
            branch: None,
            known_vector: VersionVector::new(),
            requests: Vec::new(),
        };
        let mut stats = super::ExchangeStats::default();
        let mut socket = MockSocket::new(Vec::new());

        let err = handle_server_message(
            &peer(repo_id),
            repo_id,
            &state,
            &mut socket,
            message,
            &mut stats,
        )
        .await
        .expect_err("pre-hello SyncRequest must fail closed");

        assert!(err.to_string().contains("before SyncHello"));
        assert!(socket.sent.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn p2p_exchange_rejects_configured_peer_id_mismatch() -> anyhow::Result<()> {
        let identity = Arc::new(IdentityKeyPair::generate());
        let (_dir, state) = test_state_with_dir(identity)?;
        let repo_id = uuid::Uuid::new_v4();
        let actual_peer = PeerId::new("actual-peer");
        let message = ServerMessage::SyncHello {
            peer_id: actual_peer,
            repo_id,
            scope_nonce: ScopeNonce::new(0),
            pub_key: Vec::new(),
            signature: Vec::new(),
            vector: VersionVector::new(),
        };
        let mut stats = super::ExchangeStats::default();
        let mut socket = MockSocket::new(Vec::new());

        let err = handle_server_message(
            &peer_with_id(repo_id, "expected-peer"),
            repo_id,
            &state,
            &mut socket,
            message,
            &mut stats,
        )
        .await
        .expect_err("configured peer_id mismatch must fail closed");

        assert!(err.to_string().contains("configured peer_id"));
        assert!(!stats.saw_hello);
        Ok(())
    }

    #[tokio::test]
    async fn p2p_exchange_rejects_invalid_sync_hello_signature() -> anyhow::Result<()> {
        let identity = Arc::new(IdentityKeyPair::generate());
        let (_dir, state) = test_state_with_dir(identity)?;
        let repo_id = uuid::Uuid::new_v4();
        let remote = IdentityKeyPair::generate();
        let remote_peer = remote.peer_id();
        let message = ServerMessage::SyncHello {
            peer_id: remote_peer.clone(),
            repo_id,
            scope_nonce: ScopeNonce::new(0),
            pub_key: remote.public_key_bytes().to_vec(),
            signature: vec![0; 64],
            vector: VersionVector::new(),
        };
        let mut stats = super::ExchangeStats::default();
        let mut socket = MockSocket::new(Vec::new());

        let err = handle_server_message(
            &peer_with_id(repo_id, remote_peer.as_str()),
            repo_id,
            &state,
            &mut socket,
            message,
            &mut stats,
        )
        .await
        .expect_err("invalid SyncHello signature must fail closed");

        assert!(err.to_string().contains("Handshake Signature"));
        assert!(!stats.saw_hello);
        Ok(())
    }

    #[tokio::test]
    async fn p2p_exchange_rejects_sync_hello_pubkey_peer_id_mismatch() -> anyhow::Result<()> {
        let identity = Arc::new(IdentityKeyPair::generate());
        let (_dir, state) = test_state_with_dir(identity)?;
        let repo_id = uuid::Uuid::new_v4();
        let claimed = IdentityKeyPair::generate();
        let signer = IdentityKeyPair::generate();
        let claimed_peer = claimed.peer_id();
        let vector = VersionVector::new();
        let transcript =
            sync_hello_transcript(&claimed_peer, &vector).expect("version vector serializes");
        let message = ServerMessage::SyncHello {
            peer_id: claimed_peer.clone(),
            repo_id,
            scope_nonce: ScopeNonce::new(0),
            pub_key: signer.public_key_bytes().to_vec(),
            signature: signer.sign(&transcript),
            vector,
        };
        let mut stats = super::ExchangeStats::default();
        let mut socket = MockSocket::new(Vec::new());

        let err = handle_server_message(
            &peer_with_id(repo_id, claimed_peer.as_str()),
            repo_id,
            &state,
            &mut socket,
            message,
            &mut stats,
        )
        .await
        .expect_err("SyncHello pubkey/peer_id mismatch must fail closed");

        assert!(err.to_string().contains("PeerID mismatch"));
        assert!(!stats.saw_hello);
        Ok(())
    }

    #[tokio::test]
    async fn p2p_exchange_rejects_repo_mismatch_after_sync_hello() -> anyhow::Result<()> {
        let identity = Arc::new(IdentityKeyPair::generate());
        let state = test_state(identity)?;
        let repo_id = uuid::Uuid::new_v4();
        let other_repo_id = uuid::Uuid::new_v4();
        let authenticated_peer = PeerId::new("peer-b");
        let message = ServerMessage::SyncRequest {
            repo_id: other_repo_id,
            branch: None,
            known_vector: VersionVector::new(),
            requests: Vec::new(),
        };
        let mut stats = authenticated_stats(authenticated_peer);
        let mut socket = MockSocket::new(Vec::new());

        let err = handle_server_message(
            &peer(repo_id),
            repo_id,
            &state,
            &mut socket,
            message,
            &mut stats,
        )
        .await
        .expect_err("post-hello repo mismatch must fail closed");

        assert!(err.to_string().contains("repo"));
        assert!(socket.sent.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn p2p_exchange_rejects_unoffered_sync_request_source() -> anyhow::Result<()> {
        let identity = Arc::new(IdentityKeyPair::generate());
        let state = test_state(identity)?;
        let repo_id = uuid::Uuid::new_v4();
        let unoffered_source = PeerId::new("unoffered-source");
        let message = ServerMessage::SyncRequest {
            repo_id,
            branch: None,
            known_vector: VersionVector::new(),
            requests: vec![(unoffered_source, (1, 2))],
        };
        let mut stats = authenticated_stats(PeerId::new("peer-b"));
        let mut socket = MockSocket::new(Vec::new());

        let err = handle_server_message(
            &peer(repo_id),
            repo_id,
            &state,
            &mut socket,
            message,
            &mut stats,
        )
        .await
        .expect_err("unoffered SyncRequest source must fail closed");

        assert!(err.to_string().contains("not offered"));
        assert!(socket.sent.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn p2p_exchange_rejects_unoffered_snapshot_request_source() -> anyhow::Result<()> {
        let identity = Arc::new(IdentityKeyPair::generate());
        let state = test_state(identity)?;
        let repo_id = uuid::Uuid::new_v4();
        let message = ServerMessage::SyncSnapshotRequest {
            source_peer_id: PeerId::new("unoffered-source"),
            repo_id,
            known_vector: VersionVector::new(),
            reason: Some("source-boundary-check".into()),
        };
        let mut stats = authenticated_stats(PeerId::new("peer-b"));
        let mut socket = MockSocket::new(Vec::new());

        let err = handle_server_message(
            &peer(repo_id),
            repo_id,
            &state,
            &mut socket,
            message,
            &mut stats,
        )
        .await
        .expect_err("unoffered SyncSnapshotRequest source must fail closed");

        assert!(err.to_string().contains("not offered"));
        assert!(socket.sent.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn p2p_exchange_rejects_authenticated_self_loop() -> anyhow::Result<()> {
        let identity = Arc::new(IdentityKeyPair::generate());
        let self_peer_id = identity.peer_id();
        let (_dir, state) = test_state_with_dir(identity)?;
        let repo_id = uuid::Uuid::new_v4();
        let message = ServerMessage::SyncHello {
            peer_id: self_peer_id,
            repo_id,
            scope_nonce: ScopeNonce::new(0),
            pub_key: Vec::new(),
            signature: Vec::new(),
            vector: VersionVector::new(),
        };
        let mut stats = super::ExchangeStats::default();
        let mut socket = MockSocket::new(Vec::new());

        let err = handle_server_message(
            &peer(repo_id),
            repo_id,
            &state,
            &mut socket,
            message,
            &mut stats,
        )
        .await
        .expect_err("authenticated self-loop must fail");

        assert!(err.to_string().contains("self-loop"));
        Ok(())
    }

    #[tokio::test]
    async fn p2p_exchange_waits_for_delayed_followup_after_hello() -> anyhow::Result<()> {
        let identity = Arc::new(IdentityKeyPair::generate());
        let local_peer = identity.peer_id();
        let (_dir, state) = test_state_with_dir(identity)?;
        let repo_id = state
            .repo
            .get_repo_info_for(None, Some(state.repo.local_repo_name()))?
            .expect("repo info")
            .uuid;
        append_local_op(&state, repo_id)?;
        let remote = IdentityKeyPair::generate();
        let remote_peer = remote.peer_id();
        let hello = Message::Binary(encode_server_binary(&signed_server_hello(
            &remote,
            repo_id,
            VersionVector::new(),
        ))?);
        let request = Message::Binary(encode_server_binary(&ServerMessage::SyncRequest {
            repo_id,
            branch: None,
            known_vector: VersionVector::new(),
            requests: vec![(local_peer, (1, 2))],
        })?);
        let mut socket = DelayedSocket::new(vec![
            DelayedFrame::Ready(hello),
            DelayedFrame::After {
                sleep: Box::pin(tokio::time::sleep(Duration::from_millis(900))),
                message: Some(request),
            },
        ]);

        let stats = drive_sync_exchange(
            &peer_with_id(repo_id, remote_peer.as_str()),
            repo_id,
            state,
            &mut socket,
        )
        .await?;

        assert_eq!(stats.sent_pushes, 1);
        assert_eq!(socket.sent.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn p2p_exchange_rejects_forged_sync_push_source() -> anyhow::Result<()> {
        let identity = Arc::new(IdentityKeyPair::generate());
        let state = test_state(identity)?;
        let repo_id = uuid::Uuid::new_v4();
        let authenticated_peer = PeerId::new("peer-b");
        let forged_source = PeerId::new("peer-a");
        let payload = dummy_payload();
        let message = ServerMessage::SyncPush {
            source_peer_id: forged_source.clone(),
            repo_id,
            header: SyncPushHeader::diff(repo_id, forged_source, VersionVector::new()),
            scope_nonce: ScopeNonce::new(0),
            branch: None,
            encrypted_payload: payload,
        };
        let mut stats = authenticated_stats(authenticated_peer);
        let mut socket = MockSocket::new(Vec::new());

        let err = handle_server_message(
            &peer(repo_id),
            repo_id,
            &state,
            &mut socket,
            message,
            &mut stats,
        )
        .await
        .expect_err("forged P2P SyncPush source must fail");

        assert!(err.to_string().contains("source attribution"));
        assert_eq!(stats.applied_pushes, 0);
        Ok(())
    }

    #[tokio::test]
    async fn p2p_exchange_rejects_forged_snapshot_source() -> anyhow::Result<()> {
        let identity = Arc::new(IdentityKeyPair::generate());
        let state = test_state(identity)?;
        let repo_id = uuid::Uuid::new_v4();
        let authenticated_peer = PeerId::new("peer-b");
        let forged_source = PeerId::new("peer-a");
        let message = ServerMessage::SyncPushSnapshot {
            source_peer_id: forged_source,
            repo_id,
            scope_nonce: ScopeNonce::new(0),
            branch: None,
            server_vector: VersionVector::new(),
            snapshot_kind: Some("full".into()),
            source_proof: None,
            payload: dummy_payload(),
        };
        let mut stats = authenticated_stats(authenticated_peer);
        let mut socket = MockSocket::new(Vec::new());

        let err = handle_server_message(
            &peer(repo_id),
            repo_id,
            &state,
            &mut socket,
            message,
            &mut stats,
        )
        .await
        .expect_err("forged P2P snapshot source must fail");

        assert!(err.to_string().contains("source attribution"));
        assert_eq!(stats.applied_snapshots, 0);
        Ok(())
    }

    #[test]
    fn p2p_exchange_rejects_snapshot_missing_source_proof() {
        let repo_id = uuid::Uuid::new_v4();
        let authenticated_peer = PeerId::new("peer-b");
        let stats = authenticated_stats(authenticated_peer.clone());

        let err = validate_inbound_snapshot(
            &peer(repo_id),
            repo_id,
            &stats,
            InboundSnapshotValidation {
                target_peer: &PeerId::new("local-target"),
                source_peer_id: &authenticated_peer,
                server_vector: &VersionVector::new(),
                source_proof: None,
                payload: &dummy_payload(),
            },
        )
        .expect_err("missing snapshot source proof must fail closed");

        assert!(err.to_string().contains("source proof"));
    }
}

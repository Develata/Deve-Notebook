use super::super::exchange::{MAX_EXCHANGE_FRAMES, drive_sync_exchange, handle_server_message};
use super::support::{
    DelayedFrame, DelayedSocket, MockSocket, REMOTE_PEER_ID, THIRD_PARTY_PEER_ID, append_local_op,
    authenticated_stats, peer, peer_with_id, signed_server_hello, test_state, test_state_with_dir,
};
use crate::server::p2p::ExchangeStats;
use deve_core::models::{PeerId, VersionVector};
use deve_core::protocol::ServerMessage;
use deve_core::protocol::frame::encode_server_binary;
use deve_core::security::IdentityKeyPair;
use std::sync::Arc;
use tokio::time::Duration;
use tokio_tungstenite::tungstenite::Message;

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
async fn p2p_exchange_responds_to_ping_without_aborting_handshake() -> anyhow::Result<()> {
    let identity = Arc::new(IdentityKeyPair::generate());
    let (_dir, state) = test_state_with_dir(identity)?;
    let repo_id = state
        .repo
        .get_repo_info_for(None, Some(state.repo.local_repo_name()))?
        .expect("repo info")
        .uuid;
    let remote = IdentityKeyPair::generate();
    let remote_peer = remote.peer_id();
    let hello = Message::Binary(encode_server_binary(&signed_server_hello(
        &remote,
        repo_id,
        VersionVector::new(),
    ))?);
    let mut socket = MockSocket::new(vec![Message::Ping(vec![1, 2, 3]), hello]);

    let stats = drive_sync_exchange(
        &peer_with_id(repo_id, remote_peer.as_str()),
        repo_id,
        state,
        &mut socket,
    )
    .await?;

    assert!(stats.saw_hello);
    assert_eq!(socket.sent, vec![Message::Pong(vec![1, 2, 3])]);
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
    let mut stats = ExchangeStats::default();
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
async fn p2p_exchange_rejects_duplicate_sync_hello() -> anyhow::Result<()> {
    let identity = Arc::new(IdentityKeyPair::generate());
    let (_dir, state) = test_state_with_dir(identity)?;
    let repo_id = state
        .repo
        .get_repo_info_for(None, Some(state.repo.local_repo_name()))?
        .expect("repo info")
        .uuid;
    let remote = IdentityKeyPair::generate();
    let remote_peer = remote.peer_id();
    let message = signed_server_hello(&remote, repo_id, VersionVector::new());
    let mut stats = authenticated_stats(remote_peer.clone());
    stats.allowed_export_sources = vec![state.identity_key.peer_id()];
    stats.requested_import_sources = vec![remote_peer.clone()];
    let original_allowed_export_sources = stats.allowed_export_sources.clone();
    let original_requested_import_sources = stats.requested_import_sources.clone();
    let original_authenticated_peer_id = stats.authenticated_peer_id.clone();
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
    .expect_err("duplicate SyncHello must fail closed");

    assert!(err.to_string().contains("duplicate SyncHello"));
    assert_eq!(
        stats.allowed_export_sources,
        original_allowed_export_sources
    );
    assert_eq!(
        stats.requested_import_sources,
        original_requested_import_sources
    );
    assert_eq!(stats.authenticated_peer_id, original_authenticated_peer_id);
    assert!(socket.sent.is_empty());
    Ok(())
}

#[tokio::test]
async fn p2p_exchange_rejects_repo_mismatch_after_sync_hello() -> anyhow::Result<()> {
    let identity = Arc::new(IdentityKeyPair::generate());
    let state = test_state(identity)?;
    let repo_id = uuid::Uuid::new_v4();
    let other_repo_id = uuid::Uuid::new_v4();
    let authenticated_peer = PeerId::new(REMOTE_PEER_ID);
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
    let unoffered_source = PeerId::new(THIRD_PARTY_PEER_ID);
    let message = ServerMessage::SyncRequest {
        repo_id,
        branch: None,
        known_vector: VersionVector::new(),
        requests: vec![(unoffered_source, (1_u64.into(), 2_u64.into()))],
    };
    let mut stats = authenticated_stats(PeerId::new(REMOTE_PEER_ID));
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
        source_peer_id: PeerId::new(THIRD_PARTY_PEER_ID),
        repo_id,
        known_vector: VersionVector::new(),
        reason: Some("source-boundary-check".into()),
    };
    let mut stats = authenticated_stats(PeerId::new(REMOTE_PEER_ID));
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
        requests: vec![(local_peer, (1_u64.into(), 1_u64.into()))],
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

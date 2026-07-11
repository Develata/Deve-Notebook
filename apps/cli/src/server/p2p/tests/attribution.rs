use super::super::exchange::handle_server_message;
use super::super::validation::{
    InboundSnapshotValidation, validate_inbound_push, validate_inbound_snapshot,
};
use super::support::{
    LOCAL_TARGET_PEER_ID, MockSocket, REMOTE_PEER_ID, THIRD_PARTY_PEER_ID, authenticated_stats,
    dummy_payload, peer, peer_with_id, test_state,
};
use deve_core::models::{PeerId, VersionVector};
use deve_core::protocol::{
    ScopeNonce, ServerMessage, SyncPayloadKind, SyncPushHeader, SyncSourceProof,
};
use deve_core::security::IdentityKeyPair;
use std::sync::Arc;

#[tokio::test]
async fn p2p_exchange_rejects_forged_sync_push_source() -> anyhow::Result<()> {
    let identity = Arc::new(IdentityKeyPair::generate());
    let state = test_state(identity)?;
    let repo_id = uuid::Uuid::new_v4();
    let authenticated_peer = PeerId::new(REMOTE_PEER_ID);
    let forged_source = PeerId::new(THIRD_PARTY_PEER_ID);
    let payload = dummy_payload();
    let message = ServerMessage::SyncPush {
        source_peer_id: forged_source.clone(),
        repo_id,
        range_start: 1_u64.into(),
        range_end: 1_u64.into(),
        header: SyncPushHeader::diff(repo_id, forged_source.clone(), VersionVector::new()),
        scope_nonce: ScopeNonce::new(0),
        branch: None,
        encrypted_payload: payload,
    };
    let mut stats = authenticated_stats(authenticated_peer);
    stats.requested_import_sources.push(forged_source);
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
async fn p2p_exchange_rejects_unrequested_direct_sync_push_source() -> anyhow::Result<()> {
    let identity = Arc::new(IdentityKeyPair::generate());
    let state = test_state(identity)?;
    let repo_id = uuid::Uuid::new_v4();
    let authenticated_peer = PeerId::new(REMOTE_PEER_ID);
    let payload = dummy_payload();
    let message = ServerMessage::SyncPush {
        source_peer_id: authenticated_peer.clone(),
        repo_id,
        range_start: 1_u64.into(),
        range_end: 1_u64.into(),
        header: SyncPushHeader::diff(repo_id, authenticated_peer.clone(), VersionVector::new()),
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
    .expect_err("unrequested direct P2P SyncPush source must fail closed");

    assert!(err.to_string().contains("not requested"));
    assert_eq!(stats.applied_pushes, 0);
    Ok(())
}

#[tokio::test]
async fn p2p_exchange_rejects_forged_snapshot_source() -> anyhow::Result<()> {
    let identity = Arc::new(IdentityKeyPair::generate());
    let state = test_state(identity)?;
    let repo_id = uuid::Uuid::new_v4();
    let authenticated_peer = PeerId::new(REMOTE_PEER_ID);
    let forged_source = PeerId::new(THIRD_PARTY_PEER_ID);
    let message = ServerMessage::SyncPushSnapshot {
        source_peer_id: forged_source.clone(),
        repo_id,
        waterline: 1_u64.into(),
        scope_nonce: ScopeNonce::new(0),
        branch: None,
        server_vector: VersionVector::new(),
        snapshot_kind: Some("full".into()),
        source_proof: None,
        payload: dummy_payload(),
    };
    let mut stats = authenticated_stats(authenticated_peer);
    stats.requested_import_sources.push(forged_source);
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

#[tokio::test]
async fn p2p_exchange_rejects_unrequested_direct_snapshot_source() -> anyhow::Result<()> {
    let identity = Arc::new(IdentityKeyPair::generate());
    let state = test_state(identity)?;
    let repo_id = uuid::Uuid::new_v4();
    let remote = IdentityKeyPair::generate();
    let authenticated_peer = remote.peer_id();
    let server_vector = VersionVector::new();
    let payload = dummy_payload();
    let source_proof = SyncSourceProof::sign(
        repo_id,
        &authenticated_peer,
        &server_vector,
        SyncPayloadKind::Snapshot,
        &payload,
        &remote,
    )?;
    let message = ServerMessage::SyncPushSnapshot {
        source_peer_id: authenticated_peer.clone(),
        repo_id,
        waterline: 1_u64.into(),
        scope_nonce: ScopeNonce::new(0),
        branch: None,
        server_vector,
        snapshot_kind: Some("full".into()),
        source_proof: Some(source_proof),
        payload,
    };
    let mut stats = authenticated_stats(authenticated_peer.clone());
    let mut socket = MockSocket::new(Vec::new());

    let err = handle_server_message(
        &peer_with_id(repo_id, authenticated_peer.as_str()),
        repo_id,
        &state,
        &mut socket,
        message,
        &mut stats,
    )
    .await
    .expect_err("unrequested direct P2P snapshot source must fail closed");

    assert!(err.to_string().contains("not requested"));
    assert_eq!(stats.applied_snapshots, 0);
    Ok(())
}

#[test]
fn p2p_exchange_rejects_snapshot_missing_source_proof() {
    let repo_id = uuid::Uuid::new_v4();
    let authenticated_peer = PeerId::new(REMOTE_PEER_ID);
    let mut stats = authenticated_stats(authenticated_peer.clone());
    stats
        .requested_import_sources
        .push(authenticated_peer.clone());

    let err = validate_inbound_snapshot(
        &peer(repo_id),
        repo_id,
        &stats,
        InboundSnapshotValidation {
            target_peer: &PeerId::new(LOCAL_TARGET_PEER_ID),
            source_peer_id: &authenticated_peer,
            server_vector: &VersionVector::new(),
            source_proof: None,
            payload: &dummy_payload(),
        },
    )
    .expect_err("missing snapshot source proof must fail closed");

    assert!(err.to_string().contains("source proof"));
}

#[test]
fn p2p_exchange_rejects_sync_push_missing_source_proof() {
    let repo_id = uuid::Uuid::new_v4();
    let authenticated_peer = PeerId::new(REMOTE_PEER_ID);
    let mut stats = authenticated_stats(authenticated_peer.clone());
    stats
        .requested_import_sources
        .push(authenticated_peer.clone());

    let err = validate_inbound_push(
        &peer(repo_id),
        repo_id,
        &stats,
        &PeerId::new(LOCAL_TARGET_PEER_ID),
        &authenticated_peer,
        &SyncPushHeader::diff(repo_id, authenticated_peer.clone(), VersionVector::new()),
        &dummy_payload(),
    )
    .expect_err("missing sync push source proof must fail closed");

    assert!(err.to_string().contains("source proof"));
}

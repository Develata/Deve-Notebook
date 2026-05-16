//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

use super::handlers::sync::{
    SyncPushSnapshotInput, handle_sync_push_snapshot, handle_sync_snapshot_request,
};
use super::sync_transfer_scope_test_support::{
    bound_session, build_state, encrypted_insert_for_author, recv_protocol_error,
    remote_insert_entry, unicast_channel,
};
use deve_core::models::{PeerId, VersionVector};
use deve_core::protocol::{ServerErrorCode, SyncPayloadKind, SyncSourceProof};
use deve_core::security::{EncryptedOp, IdentityKeyPair};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_push_snapshot_uses_message_source_peer_for_shadow_replace() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let relay_peer = PeerId::new("relay-peer");
    let source_key = IdentityKeyPair::generate();
    let source_peer = source_key.peer_id();
    let op = encrypted_insert_for_author(&state, repo_id, &source_peer, 1)?;
    let server_vector = VersionVector::new();
    let source_proof = signed_snapshot_source_proof(
        repo_id,
        &source_key,
        &server_vector,
        std::slice::from_ref(&op),
    )?;
    let (ch, _rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(relay_peer.clone()), Some(43));
    session.set_requested_sync_sources([source_peer.clone()]);

    handle_sync_push_snapshot(
        &state,
        &ch,
        &mut session,
        SyncPushSnapshotInput {
            peer_id: source_peer.clone(),
            repo_id,
            server_vector,
            source_proof: Some(source_proof),
            ops: vec![op],
        },
    )
    .await;

    assert_eq!(state.repo.get_shadow_max_seq(&source_peer, &repo_id)?, 1);
    assert_eq!(state.repo.get_shadow_max_seq(&relay_peer, &repo_id)?, 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_push_snapshot_rejects_relay_forged_source_proof() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let relay_key = IdentityKeyPair::generate();
    let relay_peer = relay_key.peer_id();
    let source_key = IdentityKeyPair::generate();
    let source_peer = source_key.peer_id();
    let op = encrypted_insert_for_author(&state, repo_id, &source_peer, 1)?;
    let server_vector = VersionVector::new();
    let forged_proof = signed_snapshot_source_proof(
        repo_id,
        &relay_key,
        &server_vector,
        std::slice::from_ref(&op),
    )?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(relay_peer), Some(43));
    session.set_requested_sync_sources([source_peer.clone()]);

    handle_sync_push_snapshot(
        &state,
        &ch,
        &mut session,
        SyncPushSnapshotInput {
            peer_id: source_peer.clone(),
            repo_id,
            server_vector,
            source_proof: Some(forged_proof),
            ops: vec![op],
        },
    )
    .await;

    let error = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::SyncInvalidPayload);
    assert_eq!(state.repo.get_shadow_max_seq(&source_peer, &repo_id)?, 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_request_exports_requested_shadow_source() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let relay_peer = PeerId::new("relay-peer");
    let source_peer = PeerId::new("origin-peer");
    let entry = remote_insert_entry(&source_peer, 1);
    state
        .repo
        .append_remote_ops(&source_peer, &repo_id, &[entry])?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(relay_peer), Some(47));
    session.set_offered_sync_sources([source_peer.clone()]);

    handle_sync_snapshot_request(
        &state,
        &ch,
        &mut session,
        source_peer.clone(),
        repo_id,
        Some("client-requested-full-snapshot".to_string()),
    )
    .await;

    match rx.recv().await {
        Some(deve_core::protocol::ServerMessage::SyncPushSnapshot {
            source_peer_id,
            scope_nonce,
            server_vector,
            snapshot_kind,
            payload,
            ..
        }) => {
            assert_eq!(source_peer_id, source_peer);
            assert_eq!(scope_nonce.get(), 47);
            assert_eq!(server_vector.get(&source_peer), 1);
            assert_eq!(snapshot_kind.as_deref(), Some("full"));
            assert_eq!(payload.len(), 1);
        }
        other => panic!("expected SyncPushSnapshot, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_request_rejects_unoffered_source() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let relay_peer = PeerId::new("relay-peer");
    let source_peer = PeerId::new("origin-peer");
    let entry = remote_insert_entry(&source_peer, 1);
    state
        .repo
        .append_remote_ops(&source_peer, &repo_id, &[entry])?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(relay_peer), Some(49));

    handle_sync_snapshot_request(&state, &ch, &mut session, source_peer, repo_id, None).await;

    let error = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
    Ok(())
}

fn signed_snapshot_source_proof(
    repo_id: uuid::Uuid,
    source_key: &IdentityKeyPair,
    server_vector: &VersionVector,
    payload: &[EncryptedOp],
) -> Result<SyncSourceProof, deve_core::protocol::SyncSourceProofError> {
    SyncSourceProof::sign(
        repo_id,
        &source_key.peer_id(),
        server_vector,
        SyncPayloadKind::Snapshot,
        payload,
        source_key,
    )
}

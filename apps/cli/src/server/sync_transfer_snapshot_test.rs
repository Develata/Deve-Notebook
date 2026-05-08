//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

use super::handlers::sync::{handle_sync_push_snapshot, handle_sync_snapshot_request};
use super::sync_transfer_scope_test_support::{
    bound_session, build_state, encrypted_insert_for_author, recv_protocol_error,
    remote_insert_entry, unicast_channel,
};
use deve_core::models::PeerId;
use deve_core::protocol::ServerErrorCode;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_push_snapshot_uses_message_source_peer_for_shadow_replace() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let relay_peer = PeerId::new("relay-peer");
    let source_peer = PeerId::new("origin-peer");
    let op = encrypted_insert_for_author(&state, repo_id, &source_peer, 1)?;
    let (ch, _rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(relay_peer.clone()), Some(43));
    session.set_requested_sync_sources([source_peer.clone()]);

    handle_sync_push_snapshot(
        &state,
        &ch,
        &mut session,
        source_peer.clone(),
        repo_id,
        vec![op],
    )
    .await;

    assert_eq!(state.repo.get_shadow_max_seq(&source_peer, &repo_id)?, 1);
    assert_eq!(state.repo.get_shadow_max_seq(&relay_peer, &repo_id)?, 0);
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
            peer_id,
            scope_nonce,
            server_vector,
            snapshot_kind,
            ops,
            ..
        }) => {
            assert_eq!(peer_id, source_peer);
            assert_eq!(scope_nonce, 47);
            assert_eq!(server_vector.get(&source_peer), 1);
            assert_eq!(snapshot_kind.as_deref(), Some("full"));
            assert_eq!(ops.len(), 1);
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

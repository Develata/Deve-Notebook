//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

use super::handlers::sync::{handle_sync_request, handle_sync_snapshot_request};
use super::sync_transfer_scope_test_support::{
    append_local_doc, assert_sync_binding_cleared, bound_session, build_state,
    recv_protocol_error, recv_sync_push_nonce, recv_sync_push_peer_nonce,
    recv_sync_snapshot_nonce, sync_range, unicast_channel,
};
use deve_core::models::PeerId;
use deve_core::protocol::ServerErrorCode;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_browser_sync_request_uses_bound_sync_scope_nonce_for_push() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    append_local_doc(&state)?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(PeerId::new("peer-a")), Some(17));
    session.set_offered_sync_sources([PeerId::new("test-peer")]);

    handle_sync_request(&state, &ch, &mut session, repo_id, sync_range()).await;

    assert_eq!(recv_sync_push_nonce(&mut rx).await, 17);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_browser_snapshot_request_uses_bound_sync_scope_nonce_for_push() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    append_local_doc(&state)?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(PeerId::new("peer-a")), Some(19));
    session.set_offered_sync_sources([PeerId::new("peer-a")]);

    handle_sync_snapshot_request(&state, &ch, &mut session, PeerId::new("peer-a"), repo_id, None)
        .await;

    assert_eq!(recv_sync_snapshot_nonce(&mut rx).await, 19);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_browser_sync_request_rejects_missing_sync_scope_nonce() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    append_local_doc(&state)?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(PeerId::new("peer-a")), None);
    session.set_offered_sync_sources([PeerId::new("test-peer")]);

    handle_sync_request(&state, &ch, &mut session, repo_id, sync_range()).await;

    let error = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert_eq!(error.detail.as_deref(), Some("sync scope nonce not bound"));
    assert_sync_binding_cleared(&session);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_browser_sync_request_rejects_missing_authenticated_peer_even_when_repo_bound()
-> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    append_local_doc(&state)?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, None, Some(23));

    handle_sync_request(&state, &ch, &mut session, repo_id, sync_range()).await;

    let error = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
    assert_sync_binding_cleared(&session);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_request_preserves_requested_source_peer_in_push() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    append_local_doc(&state)?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(PeerId::new("peer-a")), Some(37));
    session.set_offered_sync_sources([PeerId::new("test-peer")]);

    handle_sync_request(&state, &ch, &mut session, repo_id, sync_range()).await;

    let (peer_id, nonce) = recv_sync_push_peer_nonce(&mut rx).await;
    assert_eq!(peer_id, PeerId::new("test-peer"));
    assert_eq!(nonce, 37);
    Ok(())
}

//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

use super::handlers::sync::{
    handle_register_writer, handle_sync_request, handle_sync_snapshot_request,
};
use super::sync_scope_cleanup_test_support::{
    assert_runtime_binding_cleared, browser_session_without_sync_scope, build_state,
    recv_protocol_error, try_recv_protocol_error, unicast_channel,
};
use deve_core::models::PeerId;
use deve_core::protocol::ServerErrorCode;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_request_rejects_missing_sync_scope_nonce() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session_without_sync_scope(&state, repo_id, 17)?;

    handle_sync_request(&state, &ch, &mut session, repo_id, vec![]).await;

    let (error, scope_nonce) = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert_eq!(scope_nonce, Some(17));
    assert_runtime_binding_cleared(&session);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_snapshot_request_rejects_missing_sync_scope_nonce() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session_without_sync_scope(&state, repo_id, 19)?;

    handle_sync_snapshot_request(&state, &ch, &mut session, PeerId::new("browser"), repo_id).await;

    let (error, scope_nonce) = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert_eq!(scope_nonce, Some(19));
    assert_runtime_binding_cleared(&session);
    Ok(())
}

#[test]
fn browser_writer_registration_rejects_stale_scope_nonce_with_scoped_error() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session_without_sync_scope(&state, repo_id, 13)?;
    session.set_sync_scope_nonce(13);

    handle_register_writer(&ch, &mut session, repo_id, PeerId::new("browser"), 11);

    let (error, scope_nonce) = try_recv_protocol_error(&mut rx);
    assert_eq!(error.code, ServerErrorCode::ScStaleScope);
    assert_eq!(scope_nonce, Some(11));
    assert_runtime_binding_cleared(&session);
    Ok(())
}

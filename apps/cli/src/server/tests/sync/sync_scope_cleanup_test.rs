//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime

use super::handlers::sync::{handle_register_writer, handle_sync_request};
use super::sync_scope_cleanup_test_support::{
    assert_runtime_binding_cleared, build_state, recv_protocol_error, stale_unbound_session,
    unicast_channel,
};
use deve_core::models::PeerId;
use deve_core::protocol::ServerErrorCode;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_request_on_unbound_remote_clears_stale_db_and_sync_binding() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = stale_unbound_session(&state, true, 9)?;

    handle_sync_request(&state, &ch, &mut session, repo_id, vec![]).await;

    let (error, _) = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::SyncRepoUnbound);
    assert_runtime_binding_cleared(&session);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_request_on_unbound_non_browser_session_clears_stale_binding() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = stale_unbound_session(&state, false, 9)?;

    handle_sync_request(&state, &ch, &mut session, repo_id, vec![]).await;

    let (error, _) = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::SyncRepoUnbound);
    assert_runtime_binding_cleared(&session);
    Ok(())
}

#[test]
fn writer_registration_on_unbound_remote_clears_stale_db_and_sync_binding() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, _rx) = unicast_channel(&state);
    let mut session = stale_unbound_session(&state, true, 11)?;

    handle_register_writer(&state, &ch, &mut session, repo_id, PeerId::new("stale-peer"), 11);

    assert_runtime_binding_cleared(&session);
    Ok(())
}

//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime

use super::handlers::sync::handle_sync_hello;
use super::sync_hello_test_support::{
    assert_runtime_binding_cleared, block_shadow_peer_dir, build_state, empty_session,
    recv_protocol_error, signed_hello_for_repo, signed_hello_for_scope, unicast_channel,
};
use deve_core::protocol::ServerErrorCode;
use deve_core::security::IdentityKeyPair;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_shadow_binding_failure_clears_existing_runtime_binding() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    block_shadow_peer_dir(&state, &remote)?;
    let hello = signed_hello_for_scope(&remote, repo_id, 7);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();
    session.set_authenticated(remote.peer_id());
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(7);

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    let _ = recv_protocol_error(&mut rx).await;
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_rejects_non_browser_repo_rebinding() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello_for_repo(&remote, uuid::Uuid::new_v4());
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();
    session.switch_repo("notes".into(), Some(repo_id));
    session.set_active_db(state.repo.open_database(None, "notes")?);
    session.set_authenticated(remote.peer_id());
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(3);

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    let error = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert!(
        error
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("requested_repo_id")),
        "unexpected detail: {:?}",
        error.detail
    );
    assert_runtime_binding_cleared(&session);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_rejects_non_browser_peer_rebinding() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let current_peer = IdentityKeyPair::generate();
    let incoming_peer = IdentityKeyPair::generate();
    let hello = signed_hello_for_repo(&incoming_peer, repo_id);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();
    session.set_authenticated(current_peer.peer_id());
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(3);

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    let error = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert!(
        error
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("requested_peer_id")),
        "unexpected detail: {:?}",
        error.detail
    );
    assert_runtime_binding_cleared(&session);
    Ok(())
}

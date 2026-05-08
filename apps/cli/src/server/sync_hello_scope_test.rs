//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

use super::handlers::sync::handle_sync_hello;
use super::sync_hello_test_support::{
    assert_runtime_binding_cleared, build_state, empty_session, recv_protocol_error,
    signed_hello_for_repo, signed_hello_for_scope, unicast_channel,
};
use deve_core::protocol::{ServerError, ServerErrorCode};
use deve_core::security::IdentityKeyPair;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_rejects_non_browser_active_branch_peer_mismatch() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let current_peer = IdentityKeyPair::generate();
    let incoming_peer = IdentityKeyPair::generate();
    let hello = signed_hello_for_repo(&incoming_peer, repo_id);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();
    session.switch_branch(Some(current_peer.peer_id().to_string()));
    session.set_authenticated(current_peer.peer_id());
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(3);
    session.set_writer_identity(repo_id, current_peer.peer_id(), 3);

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    assert_repo_context_error(recv_protocol_error(&mut rx).await, "active_branch");
    assert_runtime_binding_cleared(&session);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_rejects_non_browser_stale_sync_scope_nonce_rebind() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello_for_scope(&remote, repo_id, 9);
    let repo_name = state.repo.local_repo_name().to_string();
    let local_handle = state
        .repo
        .open_database(None, state.repo.local_repo_name())?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();
    session.switch_repo(repo_name.clone(), Some(repo_id));
    session.set_active_db(local_handle);
    session.set_authenticated(remote.peer_id());
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(3);

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    assert_stale_scope_error(recv_protocol_error(&mut rx).await, "current_sync_scope_nonce");
    assert_repo_selector_preserved_without_runtime(&session, &repo_name, repo_id);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_rejects_non_browser_unresolved_active_repo_selector() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello_for_repo(&remote, repo_id);
    let local_handle = state
        .repo
        .open_database(None, state.repo.local_repo_name())?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();
    session.switch_repo("stale-notes".into(), None);
    session.set_active_db(local_handle);
    session.set_authenticated(remote.peer_id());
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(5);

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    assert_repo_context_error(recv_protocol_error(&mut rx).await, "selector not resolved");
    assert_runtime_binding_cleared(&session);
    Ok(())
}

fn assert_repo_context_error(error: ServerError, detail: &str) {
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert!(
        error
            .detail
            .as_deref()
            .is_some_and(|value| value.contains(detail)),
        "unexpected detail: {:?}",
        error.detail
    );
}

fn assert_stale_scope_error(error: ServerError, detail: &str) {
    assert_eq!(error.code, ServerErrorCode::ScStaleScope);
    assert!(
        error
            .detail
            .as_deref()
            .is_some_and(|value| value.contains(detail)),
        "unexpected detail: {:?}",
        error.detail
    );
}

fn assert_repo_selector_preserved_without_runtime(
    session: &super::session::WsSession,
    repo_name: &str,
    repo_id: uuid::Uuid,
) {
    assert_eq!(session.active_repo.as_deref(), Some(repo_name));
    assert_eq!(session.active_repo_id, Some(repo_id));
    assert!(session.get_active_db().is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
}

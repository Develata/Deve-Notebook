//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime

use super::handlers::sync::handle_sync_hello;
use super::source_control_grants::{AuthSessionId, SourceControlGrantBranch};
use super::sync_hello_test_support::{
    build_state, empty_session, signed_hello_for_scope, unicast_channel,
};
use deve_core::ledger::database::DatabaseHandle;
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use deve_core::security::IdentityKeyPair;
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_hello_rejects_stale_active_db_binding() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello_for_scope(&remote, repo_id, 9);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session(repo_id, &remote);
    let db_dir = tempfile::tempdir().expect("tempdir");
    let db = Arc::new(redb::Database::create(db_dir.path().join("stale.redb"))?);
    session.set_active_db(DatabaseHandle {
        db,
        readonly: false,
        branch: None,
        repo_id: Some(uuid::Uuid::new_v4()),
        repo_name: "notes".into(),
    });

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    assert_runtime_mismatch(recv_protocol_error(&mut rx).await);
    assert_browser_sync_binding_cleared(&session);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_hello_rejects_stale_bound_repo_and_writer_identity() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello_for_scope(&remote, repo_id, 9);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session(repo_id, &remote);
    let stale_repo_id = uuid::Uuid::new_v4();
    session.bind_repo(stale_repo_id);
    session.set_writer_identity(stale_repo_id, remote.peer_id(), 9);

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    assert_runtime_mismatch(recv_protocol_error(&mut rx).await);
    assert_browser_sync_binding_cleared(&session);
    assert!(session.writer_identity.is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_hello_failure_revokes_source_control_write_grant() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let auth_session = AuthSessionId::for_test("browser-sync-hello-failure");
    let mut session = browser_session(repo_id, &remote);
    session.bind_auth_session(auth_session.clone());
    state
        .source_control_write_grants()
        .grant(
            auth_session.clone(),
            repo_id,
            SourceControlGrantBranch::Local,
            remote.peer_id(),
            9,
        )
        .expect("grant setup");
    assert!(state
        .source_control_write_grants()
        .authorize_browser_local(&auth_session, repo_id, 9)
        .is_ok());

    let stale_hello = signed_hello_for_scope(&remote, repo_id, 8);
    let (ch, mut rx) = unicast_channel(&state);

    handle_sync_hello(&state, &ch, &mut session, stale_hello).await;

    let (error, scope_nonce) = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::ScStaleScope);
    assert_eq!(scope_nonce, Some(8));
    assert_browser_sync_binding_cleared(&session);
    let err = state
        .source_control_write_grants()
        .authorize_browser_local(&auth_session, repo_id, 9)
        .expect_err("Browser SyncHello failure must revoke the active HTTP write grant");
    assert_eq!(err.code, ServerErrorCode::ScStaleScope);
    Ok(())
}

fn browser_session(repo_id: uuid::Uuid, remote: &IdentityKeyPair) -> super::session::WsSession {
    let mut session = empty_session();
    session.mark_browser_session();
    session.switch_repo("notes".into(), Some(repo_id));
    session.set_scope_nonce(Some(9));
    session.set_sync_scope_nonce(9);
    session.set_authenticated(remote.peer_id());
    session.bind_repo(repo_id);
    session
}

async fn recv_protocol_error(rx: &mut mpsc::Receiver<ServerMessage>) -> (ServerError, Option<u64>) {
    match rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => (error, scope_nonce),
        other => panic!("expected ProtocolError, got {:?}", other),
    }
}

fn assert_runtime_mismatch((error, scope_nonce): (ServerError, Option<u64>)) {
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert_eq!(scope_nonce, Some(9));
    assert!(error
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("runtime binding mismatch")));
}

fn assert_browser_sync_binding_cleared(session: &super::session::WsSession) {
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
}

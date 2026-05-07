//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

use super::handlers::sync::handle_sync_hello;
use super::sync_hello_test_support::{
    build_state, collect_unicast_messages, empty_session, signed_hello_for_repo,
    signed_hello_for_scope, unicast_channel,
};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::security::IdentityKeyPair;

fn browser_session(repo_id: uuid::Uuid) -> super::session::WsSession {
    let mut session = empty_session();
    session.mark_browser_session();
    session.switch_repo("notes".into(), Some(repo_id));
    session.set_scope_nonce(Some(1));
    session
}

fn assert_first_sync_hello(messages: &[ServerMessage]) {
    assert!(matches!(
        messages.first(),
        Some(ServerMessage::SyncHello { .. })
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_hello_does_not_create_shadow_repo() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello_for_repo(&remote, repo_id);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session(repo_id);

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let _ = rx.recv().await;

    let shadow_dir = state.repo.remotes_dir().join(remote.peer_id().to_filename());
    assert!(!shadow_dir.try_exists()?);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_hello_skips_sync_payload_messages() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello_for_repo(&remote, repo_id);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session(repo_id);

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let messages = collect_unicast_messages(&mut rx).await?;

    assert_first_sync_hello(&messages);
    assert!(!messages.iter().any(is_sync_payload_message));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_hello_refreshes_shadow_list_without_self_peer() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    state
        .repo
        .ensure_shadow_repo_binding(&remote.peer_id(), repo_id)?;
    let hello = signed_hello_for_repo(&remote, repo_id);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session(repo_id);

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let messages = collect_unicast_messages(&mut rx).await?;

    assert_first_sync_hello(&messages);
    let shadow_list = messages
        .into_iter()
        .find_map(|msg| match msg {
            ServerMessage::ShadowList { shadows, .. } => Some(shadows),
            _ => None,
        })
        .expect("browser sync hello should refresh shadow list");
    assert!(!shadow_list.contains(&remote.peer_id().to_string()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_hello_rejects_stale_scope_nonce() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello_for_scope(&remote, repo_id, 7);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session(repo_id);
    session.set_scope_nonce(Some(9));
    session.set_active_db(state.repo.open_database(None, "notes")?);
    session.set_authenticated(remote.peer_id());
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(5);

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    match rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScStaleScope);
            assert_eq!(scope_nonce, Some(7));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    Ok(())
}

fn is_sync_payload_message(msg: &ServerMessage) -> bool {
    matches!(
        msg,
        ServerMessage::SyncRequest { .. }
            | ServerMessage::SyncSnapshotRequest { .. }
            | ServerMessage::SyncPush { .. }
            | ServerMessage::SyncPushSnapshot { .. }
    )
}

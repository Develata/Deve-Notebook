use super::channel::DualChannel;
use super::handlers::sync::handle_sync_hello;
use deve_core::ledger::listing::RepoListing;
use deve_core::protocol::ServerMessage;
use deve_core::security::IdentityKeyPair;
use deve_core::sync::vector::VersionVector;
use tokio::sync::mpsc;
#[path = "sync_hello_test_support.rs"]
mod support;

use self::support::{build_state, collect_unicast_messages, signed_hello};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_creates_repo_scoped_shadow_without_borrowing_local_metadata()
-> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let mut hello = signed_hello(&remote, &VersionVector::new());
    hello.repo_id = repo_id;
    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = super::session::WsSession::new();

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let _ = uni_rx.recv().await;

    assert!(state.repo.list_repos(Some(&remote.peer_id()))?.is_empty());
    assert!(
        state
            .repo
            .remotes_dir()
            .join(remote.peer_id().to_filename())
            .join(format!("{repo_id}.redb"))
            .exists()
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_binds_session_sync_scope_nonce() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let mut hello = signed_hello(&remote, &VersionVector::new());
    hello.repo_id = repo_id;
    hello.scope_nonce = 9;
    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = super::session::WsSession::new();

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let _ = collect_unicast_messages(&mut uni_rx).await?;

    assert_eq!(session.sync_scope_nonce(), Some(9));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_rejects_unknown_repo_before_binding_session() -> anyhow::Result<()> {
    let (_dir, state, _repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let mut hello = signed_hello(&remote, &VersionVector::new());
    hello.repo_id = uuid::Uuid::new_v4();
    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = super::session::WsSession::new();

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(
                error.code,
                deve_core::protocol::ServerErrorCode::ScRepoContextInvalid
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    assert!(
        !state
            .repo
            .remotes_dir()
            .join(remote.peer_id().to_filename())
            .try_exists()?
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_fails_closed_when_shadow_binding_fails() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    std::fs::create_dir_all(state.repo.remotes_dir())?;
    std::fs::write(
        state
            .repo
            .remotes_dir()
            .join(remote.peer_id().to_filename()),
        b"blocked",
    )?;
    let mut hello = signed_hello(&remote, &VersionVector::new());
    hello.repo_id = repo_id;
    hello.scope_nonce = 7;
    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = super::session::WsSession::new();

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { .. }) => {}
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_shadow_binding_failure_clears_existing_runtime_binding() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    std::fs::create_dir_all(state.repo.remotes_dir())?;
    std::fs::write(
        state
            .repo
            .remotes_dir()
            .join(remote.peer_id().to_filename()),
        b"blocked",
    )?;
    let mut hello = signed_hello(&remote, &VersionVector::new());
    hello.repo_id = repo_id;
    hello.scope_nonce = 7;
    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = super::session::WsSession::new();
    session.set_authenticated(remote.peer_id());
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(7);

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { .. }) => {}
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_rejects_non_browser_repo_rebinding() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let mut hello = signed_hello(&remote, &VersionVector::new());
    hello.repo_id = uuid::Uuid::new_v4();
    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = super::session::WsSession::new();
    session.switch_repo("notes".into(), Some(repo_id));
    let local_handle = state.repo.open_database(None, "notes")?;
    session.set_active_db(local_handle);
    session.set_authenticated(remote.peer_id());
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(3);

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(
                error.code,
                deve_core::protocol::ServerErrorCode::ScRepoContextInvalid
            );
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("requested_repo_id")),
                "unexpected detail: {:?}",
                error.detail
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.bound_repo_id.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.active_repo.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_rejects_non_browser_peer_rebinding() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let current_peer = IdentityKeyPair::generate();
    let incoming_peer = IdentityKeyPair::generate();
    let mut hello = signed_hello(&incoming_peer, &VersionVector::new());
    hello.repo_id = repo_id;
    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = super::session::WsSession::new();
    session.set_authenticated(current_peer.peer_id());
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(3);

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(
                error.code,
                deve_core::protocol::ServerErrorCode::ScRepoContextInvalid
            );
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("requested_peer_id")),
                "unexpected detail: {:?}",
                error.detail
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.bound_repo_id.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.active_repo.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    Ok(())
}

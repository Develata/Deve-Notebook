use super::handlers::sync::{SyncHelloInput, handle_sync_hello};
use super::{AppState, channel::DualChannel, security, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::protocol::ServerMessage;
use deve_core::security::IdentityKeyPair;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use deve_core::sync::vector::VersionVector;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:test:notes"))?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let (tx, _rx) = broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
            tx,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                identity_key.peer_id(),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_service: None,
            identity_key,
        }),
        repo_id,
    ))
}

fn signed_hello(remote: &IdentityKeyPair, vector: &VersionVector) -> SyncHelloInput {
    let peer_id = remote.peer_id();
    let sorted_map: std::collections::BTreeMap<_, _> = vector.iter().collect();
    let vec_bytes = serde_json::to_vec(&sorted_map).expect("serialize vector");
    let mut msg = Vec::new();
    msg.extend_from_slice(b"deve-handshake");
    msg.extend_from_slice(peer_id.as_str().as_bytes());
    msg.extend_from_slice(&vec_bytes);
    SyncHelloInput {
        peer_id,
        pub_key: remote.public_key_bytes().to_vec(),
        signature: remote.sign(&msg),
        remote_vector: vector.clone(),
        repo_id: uuid::Uuid::nil(),
        scope_nonce: 1,
    }
}

async fn collect_unicast_messages(
    rx: &mut mpsc::Receiver<ServerMessage>,
) -> anyhow::Result<Vec<ServerMessage>> {
    let first = rx.recv().await.expect("at least one message");
    let mut messages = vec![first];
    while let Ok(msg) = rx.try_recv() {
        messages.push(msg);
    }
    Ok(messages)
}

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

    assert_eq!(
        state.repo.list_repos(Some(&remote.peer_id()))?,
        vec![repo_id.to_string()]
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
    assert!(state.repo.list_repos(Some(&remote.peer_id()))?.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_fails_closed_when_shadow_binding_fails() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    std::fs::create_dir_all(state.repo.remotes_dir())?;
    std::fs::write(state.repo.remotes_dir().join(remote.peer_id().to_filename()), b"blocked")?;
    let mut hello = signed_hello(&remote, &VersionVector::new());
    hello.repo_id = repo_id;
    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = super::session::WsSession::new();

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(
                error.code,
                deve_core::protocol::ServerErrorCode::StoragePersistFailed
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    Ok(())
}

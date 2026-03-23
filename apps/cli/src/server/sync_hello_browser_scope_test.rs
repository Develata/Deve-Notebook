use super::handlers::sync::{SyncHelloInput, handle_sync_hello};
use super::{AppState, channel::DualChannel, security, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::database::DatabaseHandle;
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

fn signed_hello(remote: &IdentityKeyPair, repo_id: uuid::Uuid) -> SyncHelloInput {
    let vector = VersionVector::new();
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
        remote_vector: vector,
        repo_id,
        scope_nonce: 9,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_hello_rejects_stale_active_db_binding() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello(&remote, repo_id);
    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = super::session::WsSession::new();
    session.mark_browser_session();
    session.switch_repo("notes".into(), Some(repo_id));
    session.set_scope_nonce(Some(9));
    session.set_sync_scope_nonce(9);
    session.set_authenticated(remote.peer_id());
    session.bind_repo(repo_id);
    let db_dir = tempfile::tempdir().expect("tempdir");
    let db = Arc::new(redb::Database::create(db_dir.path().join("stale.redb")).expect("db"));
    session.set_active_db(DatabaseHandle {
        db,
        readonly: false,
        branch: None,
        repo_id: Some(uuid::Uuid::new_v4()),
        repo_name: "notes".into(),
    });

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(
                error.code,
                deve_core::protocol::ServerErrorCode::ScRepoContextInvalid
            );
            assert_eq!(scope_nonce, Some(9));
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("runtime binding mismatch"))
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_hello_rejects_stale_bound_repo_and_writer_identity() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello(&remote, repo_id);
    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = super::session::WsSession::new();
    session.mark_browser_session();
    session.switch_repo("notes".into(), Some(repo_id));
    session.set_scope_nonce(Some(9));
    session.set_sync_scope_nonce(9);
    session.set_authenticated(remote.peer_id());
    let stale_repo_id = uuid::Uuid::new_v4();
    session.bind_repo(stale_repo_id);
    session.set_writer_identity(stale_repo_id, remote.peer_id());

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(
                error.code,
                deve_core::protocol::ServerErrorCode::ScRepoContextInvalid
            );
            assert_eq!(scope_nonce, Some(9));
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("runtime binding mismatch"))
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.writer_identity.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    Ok(())
}

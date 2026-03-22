use super::handlers::key_exchange::handle_request_key;
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:test:notes"))?;
    repo.set_vault_root(&vault);
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(8);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
            tx,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("test-peer"),
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_key_rejects_non_browser_sessions() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo("notes".into(), Some(repo_id));
    session.bind_repo(repo_id);

    handle_request_key(&state, &ch, &mut session).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(scope_nonce, None);
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_key_uses_current_browser_scope_when_sync_scope_is_stale() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo("notes".into(), Some(repo_id));
    session.bind_repo(repo_id);
    session.set_scope_nonce(Some(17));
    session.set_sync_scope_nonce(9);

    handle_request_key(&state, &ch, &mut session).await;

    match uni_rx.recv().await {
        Some(ServerMessage::KeyProvide {
            repo_id: seen,
            scope_nonce,
            branch,
            ..
        }) => {
            assert_eq!(seen, repo_id);
            assert_eq!(scope_nonce, 17);
            assert_eq!(branch, None);
        }
        other => panic!("expected KeyProvide, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_key_on_remote_branch_uses_local_counterpart_keys_root() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: "shadow-notes".into(),
            url: Some("urn:test:notes".into()),
        },
    )?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(repo_id));
    session.bind_repo(repo_id);
    session.set_scope_nonce(Some(21));
    session.set_sync_scope_nonce(5);

    handle_request_key(&state, &ch, &mut session).await;

    match uni_rx.recv().await {
        Some(ServerMessage::KeyProvide {
            repo_id: seen,
            scope_nonce,
            branch,
            ..
        }) => {
            assert_eq!(seen, repo_id);
            assert_eq!(scope_nonce, 21);
            assert_eq!(branch, Some(peer_id.clone()));
        }
        other => panic!("expected KeyProvide, got {:?}", other),
    }
    assert!(state.repo.local_repo_notegit_keys_root("notes")?.exists());
    assert!(
        !state
            .repo
            .local_repo_notegit_keys_root("shadow-notes")?
            .exists()
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_key_denies_corrupt_repo_key() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let key_path = state
        .repo
        .local_repo_notegit_keys_root("notes")?
        .join("repo.key");
    std::fs::create_dir_all(
        key_path
            .parent()
            .expect("repo.key must have a parent directory"),
    )?;
    std::fs::write(&key_path, [1, 2, 3, 4])?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo("notes".into(), Some(repo_id));
    session.bind_repo(repo_id);
    session.set_scope_nonce(Some(31));
    session.set_sync_scope_nonce(9);

    handle_request_key(&state, &ch, &mut session).await;

    match uni_rx.recv().await {
        Some(ServerMessage::KeyDenied {
            repo_id: seen,
            scope_nonce,
            branch,
            error,
        }) => {
            assert_eq!(seen, Some(repo_id));
            assert_eq!(scope_nonce, 31);
            assert_eq!(branch, None);
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("Corrupt repo key"))
            );
        }
        other => panic!("expected KeyDenied, got {:?}", other),
    }
    assert_eq!(std::fs::read(key_path)?, vec![1, 2, 3, 4]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_key_on_missing_shadow_branch_clears_remote_scope() -> anyhow::Result<()> {
    let (_dir, state, _repo_id) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(41));
    session.switch_branch(Some("missing-shadow".into()));
    session.switch_repo("ghost".into(), None);

    handle_request_key(&state, &ch, &mut session).await;

    match uni_rx.recv().await {
        Some(ServerMessage::KeyDenied {
            repo_id,
            scope_nonce,
            branch,
            error,
        }) => {
            assert_eq!(repo_id, None);
            assert_eq!(scope_nonce, 41);
            assert_eq!(branch, None);
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("Remote branch not available:"))
            );
        }
        other => panic!("expected KeyDenied, got {:?}", other),
    }
    assert!(session.active_branch.is_none());
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}

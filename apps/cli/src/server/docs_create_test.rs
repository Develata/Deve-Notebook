use super::handlers::docs::handle_create_doc;
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::database::DatabaseHandle;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
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
async fn create_doc_rejects_existing_workspace_file_without_backfill() -> anyhow::Result<()> {
    let (dir, state, repo_id) = build_state()?;
    let path = dir.path().join("vault/default/external.md");
    std::fs::create_dir_all(path.parent().expect("parent"))?;
    std::fs::write(&path, "external only")?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo(state.repo.local_repo_name().to_string(), Some(repo_id));

    handle_create_doc(&state, &ch, &mut session, "external.md".into()).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::StorageConflict);
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(state.repo.get_docid("external.md")?.is_none());
    assert_eq!(std::fs::read_to_string(path)?, "external only");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_doc_rejects_stale_browser_scope_with_scoped_error() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo(
        state.repo.local_repo_name().to_string(),
        Some(uuid::Uuid::new_v4()),
    );
    session.set_scope_nonce(Some(17));
    session.bind_repo(repo_id);

    handle_create_doc(&state, &ch, &mut session, "scoped.md".into()).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(scope_nonce, Some(17));
        }
        other => panic!("expected scoped ProtocolError, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_doc_rejects_invalid_browser_path_with_scoped_error() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo(state.repo.local_repo_name().to_string(), Some(repo_id));
    session.set_scope_nonce(Some(23));

    handle_create_doc(&state, &ch, &mut session, "../escape.md".into()).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert_eq!(scope_nonce, Some(23));
        }
        other => panic!("expected scoped ProtocolError, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_doc_ignores_stale_remote_readonly_binding_after_scope_recovery()
-> anyhow::Result<()> {
    let (dir, state, repo_id) = build_state()?;
    let stale_db = Arc::new(redb::Database::create(
        dir.path().join("stale-remote.redb"),
    )?);
    let (uni_tx, _uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo(state.repo.local_repo_name().to_string(), Some(repo_id));
    session.set_scope_nonce(Some(31));
    session.set_active_db(DatabaseHandle {
        db: stale_db,
        readonly: true,
        branch: Some(PeerId::new("remote")),
        repo_id: Some(uuid::Uuid::new_v4()),
        repo_name: "shadow".into(),
    });

    handle_create_doc(&state, &ch, &mut session, "notes/local.md".into()).await;

    assert!(session.get_active_db().is_none());
    assert!(
        state.repo.get_docid("notes/local.md")?.is_some(),
        "local create should succeed after stale readonly binding is cleared"
    );
    assert!(dir.path().join("vault/default/notes/local.md").exists());
    Ok(())
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_doc_fails_closed_when_target_path_is_unstatable() -> anyhow::Result<()> {
    let (dir, state, repo_id) = build_state()?;
    let blocked = dir.path().join("vault/default/blocked");
    std::fs::create_dir_all(&blocked)?;
    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o000))?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo(state.repo.local_repo_name().to_string(), Some(repo_id));

    handle_create_doc(&state, &ch, &mut session, "blocked/new.md".into()).await;

    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o755))?;
    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert!(
                error
                    .detail
                    .as_deref()
                    .unwrap_or_default()
                    .contains("Failed to check create target"),
                "unexpected detail: {:?}",
                error.detail
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(state.repo.get_docid("blocked/new.md")?.is_none());
    Ok(())
}

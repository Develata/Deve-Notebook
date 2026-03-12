use super::repo_scope::{resolve_session_repo, resolve_session_repo_and_sync};
use super::{AppState, session::WsSession, tree_state::RepoTreeRegistry};
use crate::server::security;
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::broadcast;

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let default_id = repo.get_repo_info()?.expect("default info").uuid;
    let test_repo = RepoManager::init(dir.path(), 10, Some("test"), Some("urn:test"))?;
    let test_id = test_repo.get_repo_info()?.expect("test info").uuid;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(32);
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
        default_id,
        test_id,
    ))
}

#[test]
fn resolve_session_repo_recovers_from_stale_local_repo_id() -> anyhow::Result<()> {
    let (_dir, state, default_id, test_id) = build_state()?;
    let mut session = WsSession::new();
    session.switch_repo("test".into(), Some(default_id));
    let resolved = resolve_session_repo(&state, &session)?;
    assert_eq!(resolved.repo_name, "test");
    assert_eq!(resolved.repo_id, test_id);
    Ok(())
}

#[test]
fn resolve_session_repo_and_sync_updates_session_binding() -> anyhow::Result<()> {
    let (_dir, state, default_id, test_id) = build_state()?;
    let mut session = WsSession::new();
    session.switch_repo("test".into(), Some(default_id));

    let resolved = resolve_session_repo_and_sync(&state, &mut session)?;

    assert_eq!(resolved.repo_name, "test");
    assert_eq!(resolved.repo_id, test_id);
    assert_eq!(session.active_repo.as_deref(), Some("test"));
    assert_eq!(session.active_repo_id, Some(test_id));
    Ok(())
}

use super::repo_scope::{
    bootstrap_local_repo, map_repo_scope_error, resolve_session_repo, resolve_session_repo_and_sync,
};
use super::{AppState, session::WsSession, tree_state::RepoTreeRegistry};
use crate::server::security;
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use deve_core::protocol::ServerErrorCode;
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
fn resolve_session_repo_rejects_stale_local_repo_id_mismatch() -> anyhow::Result<()> {
    let (_dir, state, default_id, _test_id) = build_state()?;
    let mut session = WsSession::new();
    session.switch_repo("test".into(), Some(default_id));
    let err = resolve_session_repo(&state, &session).expect_err("stale local repo must fail");
    assert!(
        err.to_string()
            .contains("Local repository selector not resolved")
    );
    assert_eq!(session.active_repo.as_deref(), Some("test"));
    assert_eq!(session.active_repo_id, Some(default_id));
    Ok(())
}

#[test]
fn resolve_session_repo_and_sync_rejects_stale_local_repo_id_mismatch() -> anyhow::Result<()> {
    let (_dir, state, default_id, _test_id) = build_state()?;
    let mut session = WsSession::new();
    session.switch_repo("test".into(), Some(default_id));

    let err = resolve_session_repo_and_sync(&state, &mut session)
        .expect_err("stale local repo must fail before syncing session");
    assert!(
        err.to_string()
            .contains("Local repository selector not resolved")
    );
    assert_eq!(session.active_repo, None);
    assert_eq!(session.active_repo_id, None);
    Ok(())
}

#[test]
fn resolve_session_repo_and_sync_clears_stale_runtime_binding_after_selector_repair()
-> anyhow::Result<()> {
    let (_dir, state, default_id, test_id) = build_state()?;
    let mut session = WsSession::new();
    session.switch_repo("test".into(), Some(test_id));
    session.set_active_db(state.repo.open_database(None, "default")?);
    session.set_authenticated(PeerId::new("stale-writer"));
    session.bind_repo(default_id);
    session.set_sync_scope_nonce(17);
    session.set_writer_identity(default_id, PeerId::new("stale-writer"));

    let resolved = resolve_session_repo_and_sync(&state, &mut session)?;
    assert_eq!(resolved.repo_name, "test");
    assert_eq!(resolved.repo_id, test_id);
    assert_eq!(session.active_repo.as_deref(), Some("test"));
    assert_eq!(session.active_repo_id, Some(test_id));
    assert!(session.get_active_db().is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    assert!(session.writer_identity.is_none());
    Ok(())
}

#[test]
fn resolve_session_repo_rejects_local_name_recovery_from_stale_uuid() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let mut session = WsSession::new();
    session.switch_repo("stale-name".into(), Some(test_id));
    let err = resolve_session_repo_and_sync(&state, &mut session)
        .expect_err("local stale selector must fail closed");
    assert!(
        err.to_string()
            .contains("Local repository selector not resolved")
    );
    assert_eq!(session.active_repo, None);
    assert_eq!(session.active_repo_id, None);
    Ok(())
}

#[test]
fn resolve_session_repo_rejects_unrecoverable_stale_local_repo_name() -> anyhow::Result<()> {
    let (_dir, state, _default_id, _test_id) = build_state()?;
    let mut session = WsSession::new();
    session.switch_repo("stale-name".into(), None);
    let err = resolve_session_repo(&state, &session).expect_err("stale local repo must fail");
    assert!(
        err.to_string()
            .contains("Local repository selector not resolved")
    );
    Ok(())
}

#[test]
fn bootstrap_local_repo_requires_explicit_selection_when_multiple_local_repos_exist()
-> anyhow::Result<()> {
    let (_dir, state, _default_id, _test_id) = build_state()?;
    let session = WsSession::new();
    let err = bootstrap_local_repo(&state, &session).expect_err("multi repo bootstrap must fail");
    assert!(err.to_string().contains("Active repository not selected"));
    Ok(())
}

#[test]
fn map_repo_scope_error_marks_selector_mismatch_as_context_invalid() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Repo selector mismatch: repo_id resolved to default, repo_name resolved to test"
    ));
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn map_repo_scope_error_marks_missing_repo_as_not_found() {
    let err = map_repo_scope_error(anyhow::anyhow!("Repository not found: default"));
    assert_eq!(err.code, ServerErrorCode::StorageNotFound);
    let local_name_err =
        map_repo_scope_error(anyhow::anyhow!("Local repo not found for name test"));
    assert_eq!(local_name_err.code, ServerErrorCode::StorageNotFound);
}

#[test]
fn map_repo_scope_error_marks_locked_db_as_storage_db_locked() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Database already open. Cannot acquire lock."
    ));
    assert_eq!(err.code, ServerErrorCode::StorageDbLocked);
}

#[test]
fn map_repo_scope_error_marks_stale_nonce_as_context_invalid() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Browser SyncHello stale scope nonce: current_scope_nonce=9, requested_scope_nonce=7"
    ));
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn map_repo_scope_error_marks_remote_bootstrap_as_context_invalid() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Cannot bootstrap local repo while on remote branch"
    ));
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn resolve_session_repo_preserves_missing_local_catalog_failure() -> anyhow::Result<()> {
    let (dir, state, _default_id, _test_id) = build_state()?;
    std::fs::remove_dir_all(dir.path().join("local"))?;

    let mut session = WsSession::new();
    session.switch_repo("test".into(), None);
    let err =
        resolve_session_repo(&state, &session).expect_err("missing local catalog must fail closed");

    let detail = err.to_string();
    assert!(
        detail.contains("local repo directory missing"),
        "unexpected error detail: {detail}"
    );
    assert_eq!(
        map_repo_scope_error(anyhow::anyhow!(detail)).code,
        ServerErrorCode::StoragePersistFailed
    );
    Ok(())
}

#[test]
fn resolve_session_repo_and_sync_clears_missing_remote_branch_scope() -> anyhow::Result<()> {
    let (_dir, state, _default_id, _test_id) = build_state()?;
    let mut session = WsSession::new();
    session.switch_branch(Some("missing-shadow".into()));
    session.switch_repo("ghost".into(), None);

    let err =
        resolve_session_repo_and_sync(&state, &mut session).expect_err("missing shadow must fail");

    assert!(err.to_string().contains("Remote branch not available:"));
    assert!(session.active_branch.is_none());
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}

#[test]
fn resolve_session_repo_preserves_missing_remote_catalog_failure() -> anyhow::Result<()> {
    let (dir, state, _default_id, _test_id) = build_state()?;
    std::fs::remove_dir_all(dir.path().join("remotes"))?;

    let mut session = WsSession::new();
    session.switch_branch(Some("peer-a".into()));
    let err = resolve_session_repo(&state, &session)
        .expect_err("missing remote catalog must fail closed");

    let detail = err.to_string();
    assert!(
        detail.contains("Broken remote repo catalog"),
        "unexpected error detail: {detail}"
    );
    assert_eq!(
        map_repo_scope_error(anyhow::anyhow!(detail)).code,
        ServerErrorCode::StoragePersistFailed
    );
    Ok(())
}

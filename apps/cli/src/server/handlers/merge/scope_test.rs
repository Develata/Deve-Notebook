use super::{resolve_read_repo_id, resolve_write_repo_id};
use crate::server::{AppState, channel::DualChannel, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::database::DatabaseHandle;
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::broadcast;

fn build_state() -> anyhow::Result<(
    tempfile::TempDir,
    std::path::PathBuf,
    Arc<AppState>,
    uuid::Uuid,
)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(
        dir.path().join("ledger"),
        10,
        Some("default"),
        Some("urn:default"),
    )?;
    repo.set_vault_root(&vault);
    let default_id = repo.get_repo_info()?.expect("default info").uuid;
    let repo = Arc::new(repo);
    let state = Arc::new(AppState {
        repo: repo.clone(),
        sync_manager: Arc::new(deve_core::sync::SyncManager::new(
            repo.clone(),
            vault.clone(),
        )),
        tx: broadcast::channel(16).0,
        plugins: vec![],
        sync_engine: Arc::new(RepoScopedSyncEngine::new(
            PeerId::new("local"),
            repo,
            SyncMode::Auto,
        )),
        tree_manager: Arc::new(RepoTreeRegistry::new()),
        #[cfg(feature = "search")]
        search_service: None,
        identity_key: Arc::new(deve_core::security::IdentityKeyPair::generate()),
    });
    Ok((dir, vault, state, default_id))
}

#[test]
fn read_repo_id_uses_active_local_repo_without_sync_binding() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(
        dir.path().join("ledger"),
        10,
        Some("default"),
        Some("urn:default"),
    )?;
    repo.set_vault_root(&vault);
    let mut test_repo = RepoManager::init(dir.path().join("ledger"), 10, Some("test"), None)?;
    test_repo.set_vault_root(&vault);
    let test_id = test_repo.get_repo_info()?.expect("test info").uuid;
    let repo = Arc::new(repo);
    let state = Arc::new(AppState {
        repo: repo.clone(),
        sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
        tx: broadcast::channel(16).0,
        plugins: vec![],
        sync_engine: Arc::new(RepoScopedSyncEngine::new(
            PeerId::new("local"),
            repo,
            SyncMode::Auto,
        )),
        tree_manager: Arc::new(RepoTreeRegistry::new()),
        #[cfg(feature = "search")]
        search_service: None,
        identity_key: Arc::new(deve_core::security::IdentityKeyPair::generate()),
    });
    let ch = DualChannel::new(
        broadcast::channel(8).0,
        crate::server::ws::send::new_unicast_channel().0,
    );
    let mut session = crate::server::session::WsSession::new();
    session.switch_repo("test".into(), Some(test_id));

    assert_eq!(
        resolve_read_repo_id(&state, &ch, &mut session, None),
        Some(test_id)
    );
    assert_eq!(session.active_repo_id, Some(test_id));
    Ok(())
}

#[test]
fn read_repo_id_bootstraps_single_local_repo() -> anyhow::Result<()> {
    let (_dir, _vault, state, default_id) = build_state()?;
    let ch = DualChannel::new(
        broadcast::channel(8).0,
        crate::server::ws::send::new_unicast_channel().0,
    );
    let mut session = crate::server::session::WsSession::new();

    assert_eq!(
        resolve_read_repo_id(&state, &ch, &mut session, None),
        Some(default_id)
    );
    assert_eq!(session.active_repo.as_deref(), Some("default"));
    assert_eq!(session.active_repo_id, Some(default_id));
    Ok(())
}

#[test]
fn write_repo_id_bootstraps_single_local_repo() -> anyhow::Result<()> {
    let (_dir, _vault, state, default_id) = build_state()?;
    let ch = DualChannel::new(
        broadcast::channel(8).0,
        crate::server::ws::send::new_unicast_channel().0,
    );
    let mut session = crate::server::session::WsSession::new();

    assert_eq!(
        resolve_write_repo_id(&state, &ch, &mut session, None),
        Some(default_id)
    );
    assert_eq!(session.active_repo.as_deref(), Some("default"));
    assert_eq!(session.active_repo_id, Some(default_id));
    Ok(())
}

#[test]
fn read_repo_id_bootstraps_after_clearing_stale_local_binding() -> anyhow::Result<()> {
    let (dir, _vault, state, default_id) = build_state()?;
    let ch = DualChannel::new(
        broadcast::channel(8).0,
        crate::server::ws::send::new_unicast_channel().0,
    );
    let mut session = crate::server::session::WsSession::new();
    session.set_active_db(DatabaseHandle {
        db: Arc::new(redb::Database::create(dir.path().join("stale-local.redb"))?),
        readonly: false,
        branch: None,
        repo_id: Some(uuid::Uuid::new_v4()),
        repo_name: "ghost".into(),
    });
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(uuid::Uuid::new_v4());
    session.set_sync_scope_nonce(11);

    assert_eq!(
        resolve_read_repo_id(&state, &ch, &mut session, None),
        Some(default_id)
    );
    assert_eq!(session.active_repo.as_deref(), Some("default"));
    assert!(session.get_active_db().is_none());
    Ok(())
}

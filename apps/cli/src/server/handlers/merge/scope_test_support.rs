//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Shared merge scope test fixtures.

use crate::server::{AppState, channel::DualChannel, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tempfile::{TempDir, tempdir};
use tokio::sync::broadcast;

pub(super) fn app_state(repo: Arc<RepoManager>, vault: PathBuf) -> Arc<AppState> {
    Arc::new(AppState {
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
    })
}

pub(super) fn test_channel() -> DualChannel {
    DualChannel::new(
        broadcast::channel(8).0,
        crate::server::ws::send::new_unicast_channel().0,
    )
}

pub(super) fn init_repo(
    dir: &TempDir,
    vault: &Path,
    name: &str,
    url: Option<&str>,
) -> anyhow::Result<RepoManager> {
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, Some(name), url)?;
    repo.set_vault_root(vault);
    Ok(repo)
}

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let repo = init_repo(&dir, &vault, "default", Some("urn:default"))?;
    let default_id = repo.get_repo_info()?.expect("default info").uuid;
    Ok((dir, app_state(Arc::new(repo), vault), default_id))
}

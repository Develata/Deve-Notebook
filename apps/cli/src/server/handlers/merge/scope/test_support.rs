//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Shared merge scope test fixtures.

use crate::server::{AppState, channel::DualChannel, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::{path::Path, sync::Arc};
use tempfile::{TempDir, tempdir};
use tokio::sync::broadcast;

pub(super) fn app_state(repo: Arc<RepoManager>) -> anyhow::Result<Arc<AppState>> {
    Ok(Arc::new(AppState {
        repo: repo.clone(),
        sync_manager: Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?),
        tx: broadcast::channel(16).0,
        plugins: vec![],
        sync_engine: Arc::new(RepoScopedSyncEngine::new(
            PeerId::new("local"),
            repo,
            SyncMode::Auto,
        )),
        tree_manager: Arc::new(RepoTreeRegistry::new()),
        #[cfg(feature = "search")]
        search_available: false,
        identity_key: Arc::new(deve_core::security::IdentityKeyPair::generate()),
        git_bridge: deve_core::config::GitBridgeMode::Mirror,
    }))
}

pub(super) fn test_channel() -> DualChannel {
    DualChannel::new(
        broadcast::channel(8).0,
        crate::server::ws::send::new_unicast_channel().0,
    )
}

pub(super) fn init_repo(
    dir: &TempDir,
    projection_base: &Path,
    name: &str,
    url: Option<&str>,
) -> anyhow::Result<RepoManager> {
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, Some(name), url)?;
    repo.set_projection_base_for_all_local_repos_checked(projection_base)?;
    Ok(repo)
}

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let projection_base = dir.path().join("notes");
    let repo = init_repo(&dir, &projection_base, "default", Some("urn:default"))?;
    let default_id = repo.get_repo_info()?.expect("default info").uuid;
    Ok((dir, app_state(Arc::new(repo))?, default_id))
}

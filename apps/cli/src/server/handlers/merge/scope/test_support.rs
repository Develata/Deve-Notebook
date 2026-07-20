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
    url: Option<&str>,
) -> anyhow::Result<RepoManager> {
    Ok(crate::test_support::init_cataloged_repo_with_url(
        &dir.path().join("ledger"),
        projection_base,
        10,
        url.map(str::to_string),
    )?
    .repo)
}

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let projection_base = dir.path().join("notes");
    let cataloged = crate::test_support::init_cataloged_repo_with_url(
        &dir.path().join("ledger"),
        &projection_base,
        10,
        Some("urn:default".to_string()),
    )?;
    let default_id = cataloged.repo_id;
    Ok((dir, app_state(Arc::new(cataloged.repo))?, default_id))
}

//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#repo-scope-runtime

use super::{security, tree_state::RepoTreeRegistry, AppState};
use deve_core::config::SyncMode;
use deve_core::models::{PeerId, RepoId};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use tokio::sync::broadcast;

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, RepoId)> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let (repo, _default_id) = crate::server::catalog_repo_support::catalog_initial_repo(
        &ledger,
        "default",
        &projection_base,
        10,
        Some("urn:default"),
    )?;
    let test_id = crate::server::catalog_repo_support::catalog_additional_repo(
        &repo,
        &ledger,
        "test",
        &projection_base,
        10,
        Some("urn:test"),
    )?;
    let repo = Arc::new(repo);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?),
            tx: broadcast::channel(16).0,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("test-peer"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_available: false,
            identity_key,
        }),
        test_id,
    ))
}

pub(super) fn build_single_repo_state() -> anyhow::Result<(TempDir, Arc<AppState>, RepoId)> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let (repo, repo_id) = crate::server::catalog_repo_support::catalog_initial_repo(
        &ledger,
        "default",
        &projection_base,
        10,
        Some("urn:default"),
    )?;
    let repo = Arc::new(repo);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?),
            tx: broadcast::channel(16).0,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("test-peer"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_available: false,
            identity_key,
        }),
        repo_id,
    ))
}

//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use crate::server::{security, tree_state::RepoTreeRegistry, AppState};
use deve_core::config::SyncMode;
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use tokio::sync::broadcast;

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid, uuid::Uuid)> {
    let dir = tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let (repo, default_id) = crate::server::catalog_repo_support::catalog_initial_repo(
        &ledger_dir,
        "default",
        &projection_base,
        10,
        Some("urn:default"),
    )?;
    let test_id = crate::server::catalog_repo_support::catalog_additional_repo(
        &repo,
        &ledger_dir,
        "test",
        &projection_base,
        10,
        Some("urn:test"),
    )?;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(32);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?),
            tx,
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
        default_id,
        test_id,
    ))
}

//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use crate::server::{security, tree_state::RepoTreeRegistry, AppState};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, FactActor, Op, PeerId};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use tokio::sync::broadcast;

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
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
    let state = app_state(Arc::new(repo), dir.path().join("host"))?;
    Ok((dir, state, test_id))
}

pub(super) fn build_single_repo_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let (repo, default_id) = crate::server::catalog_repo_support::catalog_initial_repo(
        &ledger,
        "default",
        &projection_base,
        10,
        Some("urn:default"),
    )?;
    let state = app_state(Arc::new(repo), dir.path().join("host"))?;
    Ok((dir, state, default_id))
}

pub(super) fn seed_doc(
    state: &Arc<AppState>,
    repo_name: &str,
    path: &str,
    content: &str,
) -> anyhow::Result<DocId> {
    let (doc_id, _ops) = state
        .repo
        .apply_file_structure_in_local_repo(repo_name, path, None, "test")?;
    state
        .repo
        .local_fact_writer(FactActor::new("test")?)
        .append_content_in_local_repo(
            repo_name,
            doc_id,
            Op::Insert {
                pos: 0,
                content: content.into(),
            },
            1,
        )?;
    Ok(doc_id)
}

fn app_state(repo: Arc<RepoManager>, host: std::path::PathBuf) -> anyhow::Result<Arc<AppState>> {
    let identity_key = security::load_or_generate_identity_key(&host)?;
    Ok(Arc::new(AppState {
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
    }))
}

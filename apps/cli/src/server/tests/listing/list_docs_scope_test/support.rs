//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use crate::server::{AppState, security, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::broadcast;

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 10, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let mut test_repo = RepoManager::init(&ledger, 10, Some("test"), Some("urn:test"))?;
    test_repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let test_id = test_repo.get_repo_info()?.expect("test info").uuid;
    let state = app_state(Arc::new(repo), dir.path().join("host"))?;
    Ok((dir, state, test_id))
}

pub(super) fn build_single_repo_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 10, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let default_id = repo.get_repo_info()?.expect("default info").uuid;
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
    state.repo.append_generated_op_in_local_repo(
        repo_name,
        doc_id,
        PeerId::new("test-peer"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: content.into(),
                },
                1,
                PeerId::new("test-peer"),
                seq,
                None,
                None,
            )
        },
    )?;
    Ok(doc_id)
}

fn app_state(
    repo: Arc<RepoManager>,
    host: std::path::PathBuf,
) -> anyhow::Result<Arc<AppState>> {
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

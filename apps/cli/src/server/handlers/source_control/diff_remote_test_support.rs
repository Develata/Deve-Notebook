//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!
//! Remote diff test fixtures.

use crate::server::{AppState, security, tree_state::RepoTreeRegistry};
use deve_core::ledger::RepoManager;
use deve_core::ledger::traits::RepoSelector;
use deve_core::models::DocId;
use deve_core::models::PeerId;
use deve_core::protocol::ScPathTarget;
use deve_core::protocol::ServerMessage;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeStatus, SourceControlApi};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use deve_core::{config::SyncMode, sync::SyncManager};
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::broadcast;

pub(super) fn new_repo() -> anyhow::Result<(TempDir, RepoManager)> {
    let dir = tempdir()?;
    let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
    repo.set_vault_root(dir.path().join("vault"));
    Ok((dir, repo))
}

pub(super) fn build_state(dir: &TempDir, repo: RepoManager) -> anyhow::Result<Arc<AppState>> {
    let vault = dir.path().join("vault");
    let repo = Arc::new(repo);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok(Arc::new(AppState {
        repo: repo.clone(),
        sync_manager: Arc::new(SyncManager::new(repo.clone(), vault)),
        tx: broadcast::channel::<ServerMessage>(16).0,
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

pub(super) fn write_workspace_file(dir: &TempDir, path: &str, content: &str) {
    let abs = dir.path().join("vault").join("default").join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

pub(super) fn seed_pending_entry(repo: &RepoManager, entry: PendingFsEntry) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| pending_fs::upsert(db, &entry))
        .expect("seed pending entry");
}

pub(super) fn pending_entry(
    path: &str,
    doc_id: Option<DocId>,
    status: ChangeStatus,
    content: &str,
    detected_at: i64,
) -> PendingFsEntry {
    PendingFsEntry {
        path: path.into(),
        renamed_from: None,
        doc_id,
        change_type: status,
        content_hash: pending_fs::content_hash(content),
        detected_at,
        has_conflict: false,
    }
}

pub(super) fn commit_added_file(
    dir: &TempDir,
    repo: &RepoManager,
    path: &str,
    content: &str,
    message: &str,
) -> anyhow::Result<DocId> {
    write_workspace_file(dir, path, content);
    seed_pending_entry(
        repo,
        pending_entry(path, None, ChangeStatus::Added, content, 1),
    );
    let selector = RepoSelector::default();
    repo.stage_pending_in_repo(&selector, &ScPathTarget::from_path(path))?;
    repo.commit_staged_in_repo(&selector, message)?;
    Ok(repo.get_docid(path)?.expect("existing doc id"))
}

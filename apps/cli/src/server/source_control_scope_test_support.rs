use crate::server::{AppState, security, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ServerMessage;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeStatus, CommitFileDiff, CommitInfo};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos(&vault);
    let default_id = repo.get_repo_info()?.expect("default info").uuid;
    let mut test_repo = RepoManager::init(dir.path(), 10, Some("test"), Some("urn:test"))?;
    test_repo.set_projection_base_for_all_local_repos(&vault);
    let test_id = test_repo.get_repo_info()?.expect("test info").uuid;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone())),
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

pub(super) fn seed_pending(repo: &RepoManager, repo_name: &str, path: &str, content: &str) {
    repo.run_on_local_repo(repo_name, |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: path.into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash(content),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed pending");
}

pub(super) fn write_workspace_file(dir: &TempDir, repo_name: &str, path: &str, content: &str) {
    let abs = dir.path().join("vault").join(repo_name).join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

pub(super) async fn recv_history(
    rx: &mut mpsc::Receiver<ServerMessage>,
) -> (Option<uuid::Uuid>, Option<String>) {
    match rx.recv().await {
        Some(ServerMessage::CommitHistory {
            repo_id, commits, ..
        }) => (
            repo_id,
            commits
                .first()
                .map(|CommitInfo { message, .. }| message.clone()),
        ),
        other => panic!("expected CommitHistory, got {:?}", other),
    }
}

pub(super) async fn recv_commit_diff(
    rx: &mut mpsc::Receiver<ServerMessage>,
) -> (Option<RepoId>, Option<PeerId>, Option<u64>, Vec<CommitFileDiff>) {
    match rx.recv().await {
        Some(ServerMessage::CommitDiffResult {
            repo_id,
            branch,
            scope_nonce,
            diffs,
            ..
        }) => (repo_id, branch, scope_nonce, diffs),
        other => panic!("expected CommitDiffResult, got {:?}", other),
    }
}

pub(super) async fn recv_changes(
    rx: &mut mpsc::Receiver<ServerMessage>,
) -> (Option<uuid::Uuid>, Vec<String>) {
    match rx.recv().await {
        Some(ServerMessage::ChangesList {
            repo_id, unstaged, ..
        }) => (
            repo_id,
            unstaged.into_iter().map(|entry| entry.path).collect(),
        ),
        other => panic!("expected ChangesList, got {:?}", other),
    }
}

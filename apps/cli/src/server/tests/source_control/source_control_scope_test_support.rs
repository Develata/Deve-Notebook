use crate::server::{
    security,
    session::WsSession,
    source_control_grants::{AuthSessionId, SourceControlGrantBranch},
    tree_state::RepoTreeRegistry,
    AppState,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ServerMessage;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeStatus, CommitFileDiffSummary, CommitInfo};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use tokio::sync::{broadcast, mpsc};

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
    let (tx, _rx) = broadcast::channel(16);
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
    let abs = workspace_root(dir, repo_name).join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

pub(super) fn bind_browser_writer(
    state: &Arc<AppState>,
    session: &mut WsSession,
    repo_id: RepoId,
    scope_nonce: u64,
) -> anyhow::Result<()> {
    let repo_name = session
        .active_repo
        .clone()
        .ok_or_else(|| anyhow::anyhow!("browser writer fixture requires an active repo"))?;
    let auth_session_id =
        AuthSessionId::for_test(&format!("source-control:{repo_id}:{scope_nonce}"));
    session.mark_browser_session();
    session.bind_auth_session(auth_session_id.clone());
    session.switch_repo(repo_name, Some(repo_id));
    session.set_scope_nonce(Some(scope_nonce));
    session.set_sync_scope_nonce(scope_nonce);
    session.set_authenticated(PeerId::new("test-peer"));
    session.bind_repo(repo_id);
    session.mark_sync_hello_accepted();
    session.set_writer_identity(repo_id, PeerId::new("test-peer"), scope_nonce);
    state
        .source_control_write_grants()
        .grant(
            auth_session_id,
            repo_id,
            SourceControlGrantBranch::Local,
            PeerId::new("test-peer"),
            scope_nonce,
        )
        .map_err(|err| anyhow::anyhow!("source-control write grant failed: {err:?}"))?;
    Ok(())
}

fn workspace_root(dir: &TempDir, repo_selector: &str) -> std::path::PathBuf {
    let base = dir.path().join("notes");
    let locator_path = dir.path().join("ledger/.host/projection-locators.toml");
    let content = std::fs::read_to_string(&locator_path).expect("projection locator file");
    let value: toml::Value = toml::from_str(&content).expect("projection locator toml");
    let locators = value["locators"].as_array().expect("projection locators");
    let locator = locators
        .iter()
        .find(|locator| locator["repo_id"].as_str() == Some(repo_selector))
        .or_else(|| (locators.len() == 1).then(|| &locators[0]))
        .expect("repo locator");
    let workspace_segment = locator["workspace_segment"]
        .as_str()
        .expect("workspace segment");
    base.join(workspace_segment)
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
) -> (
    Option<RepoId>,
    Option<PeerId>,
    Option<u64>,
    Vec<CommitFileDiffSummary>,
) {
    match rx.recv().await {
        Some(ServerMessage::CommitDiffResult {
            repo_id,
            branch,
            scope_nonce,
            files,
            ..
        }) => (repo_id, branch, scope_nonce, files),
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

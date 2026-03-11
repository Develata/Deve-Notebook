use super::source_control_proxy::RemoteSourceControlApi;
use super::{AppState, router, security, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::models::PeerId;
use deve_core::protocol::ScPathTarget;
use deve_core::security::AuthConfig;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

fn write_workspace_file(dir: &TempDir, path: &str, content: &str) {
    let abs = dir.path().join("vault").join("default").join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

fn seed_pending(repo: &RepoManager, entry: PendingFsEntry) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| pending_fs::upsert(db, &entry))
        .expect("seed pending entry");
}

async fn spawn_proxy_server() -> anyhow::Result<(
    TempDir,
    Arc<RepoManager>,
    RemoteSourceControlApi,
    JoinHandle<()>,
)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(16);
    let state = Arc::new(AppState {
        repo: repo.clone(),
        sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
        tx,
        plugins: vec![],
        sync_engine: Arc::new(RepoScopedSyncEngine::new(
            PeerId::new("test-peer"),
            repo.clone(),
            SyncMode::Auto,
        )),
        tree_manager: Arc::new(RepoTreeRegistry::new()),
        #[cfg(feature = "search")]
        search_service: None,
        identity_key: security::load_or_generate_identity_key(&dir.path().join("host"))?,
    });
    let app = router::build_app(state, 3001, Arc::new(AuthConfig::dev_default()?))
        .into_make_service_with_connect_info::<std::net::SocketAddr>();
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let base_url = format!("http://{}", listener.local_addr()?);
    let task = tokio::spawn(async move { axum::serve(listener, app).await.expect("serve app") });
    Ok((dir, repo, RemoteSourceControlApi::new(base_url), task))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_proxy_commit_diff_reports_rename() -> anyhow::Result<()> {
    let (dir, repo, proxy, task) = spawn_proxy_server().await?;
    let selector = RepoSelector::default();
    write_workspace_file(&dir, "notes/a.md", "hello");
    seed_pending(
        &repo,
        PendingFsEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: None,
            change_type: ChangeStatus::Added,
            content_hash: pending_fs::content_hash("hello"),
            detected_at: 1,
            has_conflict: false,
        },
    );
    proxy.stage_pending_in_repo(&selector, &ScPathTarget::from_path("notes/a.md"))?;
    let first = proxy.commit_staged_in_repo(&selector, "initial")?;
    let doc_id = repo.get_docid("notes/a.md")?.expect("existing doc id");

    write_workspace_file(&dir, "notes/b.md", "hello");
    std::fs::remove_file(dir.path().join("vault/default/notes/a.md"))?;
    seed_pending(
        &repo,
        PendingFsEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Deleted,
            content_hash: String::new(),
            detected_at: 2,
            has_conflict: false,
        },
    );
    seed_pending(
        &repo,
        PendingFsEntry {
            path: "notes/b.md".into(),
            renamed_from: Some("notes/a.md".into()),
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Added,
            content_hash: pending_fs::content_hash("hello"),
            detected_at: 2,
            has_conflict: false,
        },
    );
    proxy.stage_pending_in_repo(&selector, &ScPathTarget::from_path("notes/b.md"))?;
    let second = proxy.commit_staged_in_repo(&selector, "rename")?;

    let diffs = proxy.diff_commits_in_repo(&selector, Some(&first.id), &second.id)?;
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].status, ChangeStatus::Renamed);
    assert_eq!(diffs[0].previous_path.as_deref(), Some("notes/a.md"));
    assert_eq!(diffs[0].path, "notes/b.md");
    task.abort();
    Ok(())
}

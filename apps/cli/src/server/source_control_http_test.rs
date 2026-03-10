// apps/cli/src/server/source_control_http_test.rs

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

fn seed_pending(repo: &RepoManager, path: &str, status: ChangeStatus, content: &str) {
    seed_pending_entry(
        repo,
        PendingFsEntry {
            path: path.into(),
            renamed_from: None,
            doc_id: None,
            change_type: status,
            content_hash: pending_fs::content_hash(content),
            detected_at: 1,
            has_conflict: false,
        },
    );
}

fn seed_pending_entry(repo: &RepoManager, entry: PendingFsEntry) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| pending_fs::upsert(db, &entry))
        .expect("seed pending entry");
}

fn path_target(path: &str) -> ScPathTarget {
    ScPathTarget::from_path(path)
}

fn write_workspace_file(dir: &TempDir, path: &str, content: &str) {
    let abs = dir.path().join("vault").join("default").join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

async fn spawn_proxy_server() -> anyhow::Result<(
    TempDir,
    Arc<RepoManager>,
    String,
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
        sync_manager: Arc::new(deve_core::sync::SyncManager::new(
            repo.clone(),
            vault.clone(),
        )),
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
    let addr = listener.local_addr()?;
    let base_url = format!("http://{}", addr);
    let task = tokio::spawn(async move { axum::serve(listener, app).await.expect("serve app") });
    Ok((
        dir,
        repo,
        base_url.clone(),
        RemoteSourceControlApi::new(base_url),
        task,
    ))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_proxy_unstage_roundtrip() -> anyhow::Result<()> {
    let (_dir, repo, _base_url, proxy, task) = spawn_proxy_server().await?;
    let selector = RepoSelector::default();
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    proxy.stage_pending_in_repo(&selector, &path_target("notes/a.md"))?;
    assert!(proxy.list_pending_fs_in_repo(&selector)?.is_empty());
    proxy.unstage_file_in_repo(&selector, &path_target("notes/a.md"))?;
    let pending = proxy.list_pending_fs_in_repo(&selector)?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/a.md");
    assert_eq!(pending[0].status, ChangeStatus::Added);
    task.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_proxy_commit_queries_roundtrip() -> anyhow::Result<()> {
    let (dir, repo, _base_url, proxy, task) = spawn_proxy_server().await?;
    let selector = RepoSelector::default();
    write_workspace_file(&dir, "notes/a.md", "hello");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    proxy.stage_pending_in_repo(&selector, &path_target("notes/a.md"))?;
    let c1 = proxy.commit_staged_in_repo(&selector, "c1")?;
    write_workspace_file(&dir, "notes/b.md", "world");
    seed_pending(&repo, "notes/b.md", ChangeStatus::Added, "world");
    proxy.stage_pending_in_repo(&selector, &path_target("notes/b.md"))?;
    let c2 = proxy.commit_staged_in_repo(&selector, "c2")?;
    let commits = proxy.list_commits_in_repo(&selector, 10)?;
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].id, c2.id);
    assert_eq!(commits[1].id, c1.id);
    let diffs = proxy.diff_commits_in_repo(&selector, Some(&c1.id), &c2.id)?;
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].path, "notes/b.md");
    assert_eq!(diffs[0].status, ChangeStatus::Added);
    task.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_proxy_rename_candidate_collapses_and_stages_pair() -> anyhow::Result<()> {
    let (dir, repo, _base_url, proxy, task) = spawn_proxy_server().await?;
    let selector = RepoSelector::default();
    write_workspace_file(&dir, "notes/a.md", "hello");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    proxy.stage_pending_in_repo(&selector, &path_target("notes/a.md"))?;
    proxy.commit_staged_in_repo(&selector, "initial")?;
    let doc_id = repo.get_docid("notes/a.md")?.expect("existing doc id");

    write_workspace_file(&dir, "notes/b.md", "hello");
    std::fs::remove_file(dir.path().join("vault").join("default").join("notes/a.md"))?;
    seed_pending_entry(
        &repo,
        PendingFsEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Deleted,
            content_hash: String::new(),
            detected_at: 1,
            has_conflict: false,
        },
    );
    seed_pending_entry(
        &repo,
        PendingFsEntry {
            path: "notes/b.md".into(),
            renamed_from: Some("notes/a.md".into()),
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Added,
            content_hash: pending_fs::content_hash("hello"),
            detected_at: 1,
            has_conflict: false,
        },
    );

    let pending = proxy.list_pending_fs_in_repo(&selector)?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/b.md");
    assert_eq!(pending[0].renamed_from.as_deref(), Some("notes/a.md"));

    proxy.stage_pending_in_repo(&selector, &path_target("notes/b.md"))?;
    assert!(proxy.list_pending_fs_in_repo(&selector)?.is_empty());
    task.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_http_stage_prefers_doc_id_over_stale_path() -> anyhow::Result<()> {
    let (dir, repo, base_url, _proxy, task) = spawn_proxy_server().await?;
    let selector = RepoSelector::default();
    write_workspace_file(&dir, "notes/a.md", "hello");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    repo.stage_pending_in_repo(&selector, &path_target("notes/a.md"))?;
    repo.commit_staged_in_repo(&selector, "initial")?;
    let doc_id = repo.get_docid("notes/a.md")?.expect("existing doc id");

    write_workspace_file(&dir, "notes/b.md", "world");
    std::fs::remove_file(dir.path().join("vault").join("default").join("notes/a.md"))?;
    seed_pending_entry(
        &repo,
        PendingFsEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Deleted,
            content_hash: String::new(),
            detected_at: 1,
            has_conflict: false,
        },
    );
    seed_pending_entry(
        &repo,
        PendingFsEntry {
            path: "notes/b.md".into(),
            renamed_from: Some("notes/a.md".into()),
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Added,
            content_hash: pending_fs::content_hash("world"),
            detected_at: 1,
            has_conflict: false,
        },
    );

    let response = reqwest::Client::new()
        .post(format!("{}/api/sc/stage-pending", base_url))
        .json(&serde_json::json!({
            "path": "notes/a.md",
            "doc_id": doc_id.to_string(),
        }))
        .send()
        .await?;

    assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);
    assert!(repo.list_pending_fs_in_repo(&selector)?.is_empty());
    task.abort();
    Ok(())
}

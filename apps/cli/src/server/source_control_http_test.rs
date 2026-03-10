// apps/cli/src/server/source_control_http_test.rs

use super::source_control_proxy::RemoteSourceControlApi;
use super::{AppState, router, security, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::models::PeerId;
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
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: path.into(),
                doc_id: None,
                change_type: status,
                content_hash: pending_fs::content_hash(content),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed pending entry");
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
    let task = tokio::spawn(async move { axum::serve(listener, app).await.expect("serve app") });
    Ok((
        dir,
        repo,
        RemoteSourceControlApi::new(format!("http://{}", addr)),
        task,
    ))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_proxy_unstage_roundtrip() -> anyhow::Result<()> {
    let (_dir, repo, proxy, task) = spawn_proxy_server().await?;
    let selector = RepoSelector::default();
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    proxy.stage_pending_in_repo(&selector, "notes/a.md")?;
    assert!(proxy.list_pending_fs_in_repo(&selector)?.is_empty());
    proxy.unstage_file_in_repo(&selector, "notes/a.md")?;
    let pending = proxy.list_pending_fs_in_repo(&selector)?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/a.md");
    assert_eq!(pending[0].status, ChangeStatus::Added);
    task.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_proxy_commit_queries_roundtrip() -> anyhow::Result<()> {
    let (dir, repo, proxy, task) = spawn_proxy_server().await?;
    let selector = RepoSelector::default();
    write_workspace_file(&dir, "notes/a.md", "hello");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    proxy.stage_pending_in_repo(&selector, "notes/a.md")?;
    let c1 = proxy.commit_staged_in_repo(&selector, "c1")?;
    write_workspace_file(&dir, "notes/b.md", "world");
    seed_pending(&repo, "notes/b.md", ChangeStatus::Added, "world");
    proxy.stage_pending_in_repo(&selector, "notes/b.md")?;
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

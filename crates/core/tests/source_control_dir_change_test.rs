use deve_core::ledger::RepoManager;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::sync::SyncManager;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, Arc<RepoManager>) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    (dir, Arc::new(repo))
}

fn workspace_path(repo: &RepoManager, path: &str) -> std::path::PathBuf {
    repo.local_repo_workspace_path("default", path)
        .expect("workspace path")
}

fn write_workspace_file(repo: &RepoManager, path: &str, content: &str) {
    let abs = workspace_path(repo, path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

fn seed_pending_add(repo: &RepoManager, path: &str, content: &str) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
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
    .expect("seed pending add");
}

#[test]
fn dir_change_rescan_records_child_rename_candidates() {
    let (_dir, repo) = new_repo();
    write_workspace_file(repo.as_ref(), "notes/a.md", "hello");
    seed_pending_add(repo.as_ref(), "notes/a.md", "hello");
    repo.stage_pending("notes/a.md").expect("stage file");
    repo.commit_staged("initial").expect("commit file");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup doc id")
        .expect("tracked doc");

    std::fs::rename(
        workspace_path(repo.as_ref(), "notes"),
        workspace_path(repo.as_ref(), "docs"),
    )
    .expect("rename folder");
    let sync = SyncManager::new_checked(repo.clone()).expect("sync manager");
    let repo_id = repo
        .get_repo_info_for(None, Some("default"))
        .expect("repo info lookup")
        .expect("repo info")
        .uuid;
    sync.handle_dir_change("default", repo_id, "docs")
        .expect("handle dir change")
        .expect("repo-scoped result");

    let pending = repo.list_pending_fs().expect("pending after dir change");
    assert!(pending.iter().any(|entry| {
        entry.path == "notes/a.md"
            && entry.status == ChangeStatus::Deleted
            && entry.doc_id == Some(doc_id)
    }));
    assert!(pending.iter().any(|entry| {
        entry.path == "docs/a.md"
            && entry.status == ChangeStatus::Added
            && entry.doc_id == Some(doc_id)
            && entry.renamed_from.as_deref() == Some("notes/a.md")
    }));
}

#[test]
fn dir_change_ignores_repo_root_refresh() {
    let (_dir, repo) = new_repo();
    write_workspace_file(repo.as_ref(), "notes/a.md", "hello");
    let repo_id = repo
        .get_repo_info_for(None, Some("default"))
        .expect("repo info lookup")
        .expect("repo info")
        .uuid;
    let sync = SyncManager::new_checked(repo).expect("sync manager");

    assert!(
        sync.handle_dir_change("default", repo_id, "")
            .expect("handle repo root dir change")
            .is_none()
    );
}

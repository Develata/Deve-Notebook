use deve_core::ledger::RepoManager;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::sync::scan;
use deve_core::vfs::Vfs;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos(dir.path().join("vault"));
    (dir, repo)
}

fn workspace_path(dir: &TempDir, path: &str) -> std::path::PathBuf {
    dir.path().join("vault").join("default").join(path)
}

fn write_workspace_file(dir: &TempDir, path: &str, content: &str) {
    let abs = workspace_path(dir, path);
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
fn scan_records_rename_candidate_by_inode() {
    let (dir, repo) = new_repo();
    write_workspace_file(&dir, "notes/a.md", "hello");
    seed_pending_add(&repo, "notes/a.md", "hello");
    repo.stage_pending("notes/a.md").expect("stage a");
    repo.commit_staged("initial").expect("commit a");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup doc id")
        .expect("existing doc");

    std::fs::rename(
        workspace_path(&dir, "notes/a.md"),
        workspace_path(&dir, "notes/b.md"),
    )
    .expect("rename file on disk");

    let repo = Arc::new(repo);
    let vfs = Vfs::new(dir.path().join("vault"));
    scan::scan_projection_workspaces(&repo, &vfs).expect("scan workspace");

    let pending = repo.list_pending_fs().expect("pending after scan");
    assert_eq!(pending.len(), 2);
    assert!(pending.iter().any(|entry| {
        entry.path == "notes/a.md"
            && entry.status == ChangeStatus::Deleted
            && entry.doc_id == Some(doc_id)
    }));
    assert!(pending.iter().any(|entry| {
        entry.path == "notes/b.md"
            && entry.status == ChangeStatus::Added
            && entry.doc_id == Some(doc_id)
            && entry.renamed_from.as_deref() == Some("notes/a.md")
    }));
}

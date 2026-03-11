use deve_core::ledger::RepoManager;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::sync::SyncManager;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, Arc<RepoManager>, std::path::PathBuf) {
    let dir = tempdir().expect("create tempdir");
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(&vault);
    (dir, Arc::new(repo), vault)
}

fn workspace_path(root: &std::path::Path, path: &str) -> std::path::PathBuf {
    root.join("default").join(path)
}

fn write_workspace_file(root: &std::path::Path, path: &str, content: &str) {
    let abs = workspace_path(root, path);
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
    let (_dir, repo, vault) = new_repo();
    write_workspace_file(&vault, "notes/a.md", "hello");
    seed_pending_add(repo.as_ref(), "notes/a.md", "hello");
    repo.stage_pending("notes/a.md").expect("stage file");
    repo.commit_staged("initial").expect("commit file");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup doc id")
        .expect("tracked doc");

    std::fs::rename(
        workspace_path(&vault, "notes"),
        workspace_path(&vault, "docs"),
    )
    .expect("rename folder");
    let sync = SyncManager::new(repo.clone(), vault.clone());
    sync.handle_dir_change("default/docs")
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
    let (_dir, repo, vault) = new_repo();
    write_workspace_file(&vault, "notes/a.md", "hello");
    let sync = SyncManager::new(repo, vault);

    assert!(
        sync.handle_dir_change("default")
            .expect("handle repo root dir change")
            .is_none()
    );
}

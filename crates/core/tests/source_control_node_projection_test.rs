use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::sync::scan;
use deve_core::vfs::Vfs;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    (dir, repo)
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

fn poison_metadata_path(repo: &RepoManager, doc_id: deve_core::models::DocId, stale: &str) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write_txn = db.begin_write()?;
        {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;
            p2d.remove("notes/a.md")?;
            p2d.insert(stale, doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), stale)?;
        }
        write_txn.commit()?;
        Ok(())
    })
    .expect("poison metadata path");
}

fn scan_initial(repo: RepoManager, _dir: &TempDir) -> Arc<RepoManager> {
    let repo_root = repo
        .local_repo_workspace_root("default")
        .expect("workspace root");
    let repo = Arc::new(repo);
    let vfs = Vfs::new(repo_root);
    scan::scan_projection_workspaces(&repo, &vfs).expect("scan initial file");
    repo
}

#[test]
fn discard_tracked_add_prefers_node_projection_path() {
    let (dir, repo) = new_repo();
    write_workspace_file(&repo, "notes/a.md", "hello");
    let repo = scan_initial(repo, &dir);
    repo.stage_pending("notes/a.md").expect("stage file");
    repo.apply_external_changes().expect("apply external file");
    repo.commit_source_control_changes("initial")
        .expect("commit file");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup")
        .expect("doc id");
    poison_metadata_path(&repo, doc_id, "stale/a.md");

    std::fs::remove_file(workspace_path(&repo, "notes/a.md")).expect("drop canonical file");
    write_workspace_file(&repo, "notes/b.md", "hello");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/b.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("hello"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed tracked add");

    repo.discard_pending("notes/b.md")
        .expect("discard rename add");

    assert!(workspace_path(&repo, "notes/a.md").exists());
    assert!(!workspace_path(&repo, "notes/b.md").exists());
    assert!(!workspace_path(&repo, "stale/a.md").exists());
}

#[test]
fn scan_rename_prefers_node_projection_path() {
    let (dir, repo) = new_repo();
    write_workspace_file(&repo, "notes/a.md", "hello");
    let repo = scan_initial(repo, &dir);
    repo.stage_pending("notes/a.md").expect("stage file");
    repo.apply_external_changes().expect("apply external file");
    repo.commit_source_control_changes("initial")
        .expect("commit file");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup")
        .expect("doc id");
    poison_metadata_path(&repo, doc_id, "stale/a.md");

    std::fs::rename(
        workspace_path(&repo, "notes/a.md"),
        workspace_path(&repo, "notes/b.md"),
    )
    .expect("rename file");
    let vfs = Vfs::new(
        repo.local_repo_workspace_root("default")
            .expect("workspace root"),
    );
    scan::scan_projection_workspaces(&repo, &vfs).expect("scan rename");

    let pending = repo.list_pending_fs().expect("pending after scan");
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
    assert!(!pending.iter().any(|entry| entry.path == "stale/a.md"));
}

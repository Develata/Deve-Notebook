use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    (dir, repo)
}

fn write_workspace_file(repo: &RepoManager, path: &str, content: &str) {
    let abs = repo
        .local_repo_workspace_path("default", path)
        .expect("workspace path");
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

#[test]
fn delete_commit_without_doc_hint_prefers_node_projection() {
    let (_dir, repo) = new_repo();
    write_workspace_file(&repo, "notes/a.md", "hello");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("hello"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed add");
    repo.stage_pending("notes/a.md").expect("stage add");
    repo.apply_external_changes().expect("apply external add");
    repo.commit_source_control_changes("initial")
        .expect("commit add");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup")
        .expect("doc id");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write_txn = db.begin_write()?;
        {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;
            p2d.remove("notes/a.md")?;
            p2d.insert("stale/a.md", doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), "stale/a.md")?;
        }
        write_txn.commit()?;
        Ok(())
    })
    .expect("poison metadata only");
    std::fs::remove_file(
        repo.local_repo_workspace_path("default", "notes/a.md")
            .expect("workspace path"),
    )
    .expect("remove file");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Deleted,
                content_hash: String::new(),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed delete");
    repo.stage_pending("notes/a.md").expect("stage delete");
    repo.apply_external_changes()
        .expect("apply external delete");
    repo.commit_source_control_changes("delete")
        .expect("commit delete");
    assert!(repo.get_docid("stale/a.md").expect("stale path").is_none());
}

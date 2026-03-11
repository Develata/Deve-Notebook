use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    (dir, repo)
}

fn write_workspace_file(dir: &TempDir, path: &str, content: &str) {
    let abs = dir.path().join("vault").join("default").join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

#[test]
fn commit_diff_prefers_node_projection_path_over_stale_metadata() {
    let (dir, repo) = new_repo();
    write_workspace_file(&dir, "notes/a.md", "v1");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("v1"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed initial add");
    repo.stage_pending("notes/a.md").expect("stage first");
    let first = repo.commit_staged("first").expect("commit first");
    let doc_id = repo.get_docid("notes/a.md").expect("lookup").expect("doc");

    write_workspace_file(&dir, "notes/a.md", "v2");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("v2"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed modify");
    repo.stage_pending("notes/a.md").expect("stage second");
    let second = repo.commit_staged("second").expect("commit second");

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

    let diffs = repo
        .diff_commits(Some(&first.id), &second.id)
        .expect("diff commits");
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].path, "notes/a.md");
    assert_eq!(diffs[0].status, ChangeStatus::Modified);
}

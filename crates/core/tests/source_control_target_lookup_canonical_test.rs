use deve_core::ledger::RepoManager;
use deve_core::protocol::ScPathTarget;
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
fn workdir_diff_target_rejects_doc_id_when_requested_path_is_not_in_change_set() {
    let (dir, repo) = new_repo();
    write_workspace_file(&dir, "notes/a.md", "hello");
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
    repo.commit_staged("initial").expect("commit add");

    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup")
        .expect("doc id");

    let target = ScPathTarget {
        path: "stale/a.md".into(),
        doc_id: Some(doc_id),
    };
    let err = repo
        .workdir_diff_inputs_for_target_in_local_repo(repo.local_repo_name(), &target)
        .expect_err("stale doc target must fail closed");

    assert!(
        err.to_string()
            .contains("Source control target not resolved")
    );
}

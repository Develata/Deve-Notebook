use deve_core::ledger::RepoManager;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use tempfile::tempdir;

#[test]
fn commit_staged_fails_when_workspace_file_is_missing() {
    let dir = tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/missing.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("missing"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed pending");

    repo.stage_pending("notes/missing.md")
        .expect("stage pending");
    let err = repo.commit_staged("commit missing").expect_err("must fail");
    let msg = err.to_string();
    assert!(msg.contains("Failed to read staged workspace file"));
    assert!(msg.contains("notes/missing.md"));
}

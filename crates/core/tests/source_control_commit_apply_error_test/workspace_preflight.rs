use deve_core::ledger::RepoManager;
use deve_core::ledger::range;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::staging;
use tempfile::tempdir;

#[test]
fn apply_external_changes_fails_when_workspace_file_is_missing() {
    let dir = tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");

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
    let err = repo.apply_external_changes().expect_err("must fail");
    let msg = err.to_string();
    assert!(msg.contains("Failed to read staged workspace file"));
    assert!(msg.contains("notes/missing.md"));
}

#[test]
fn apply_external_changes_preflights_all_workspace_files_before_ledger_append() {
    let dir = tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    let existing_path = repo
        .local_repo_workspace_path("default", "notes/a.md")
        .expect("workspace path");
    std::fs::create_dir_all(existing_path.parent().expect("parent")).expect("create parent");
    std::fs::write(&existing_path, "ok").expect("write existing");

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("ok"),
                detected_at: 1,
                has_conflict: false,
            },
        )?;
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/z_missing.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("missing"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed pending");
    repo.stage_pending("notes/a.md").expect("stage existing");
    repo.stage_pending("notes/z_missing.md")
        .expect("stage missing");
    let before_seq = repo
        .run_on_local_repo(repo.local_repo_name(), range::get_max_seq)
        .expect("before seq");

    let err = repo
        .apply_external_changes()
        .expect_err("preflight must reject missing file");

    let after_seq = repo
        .run_on_local_repo(repo.local_repo_name(), range::get_max_seq)
        .expect("after seq");
    let staged = repo
        .run_on_local_repo(repo.local_repo_name(), staging::list_staged_entries)
        .expect("staged retained");
    assert!(err.to_string().contains("notes/z_missing.md"));
    assert_eq!(after_seq, before_seq);
    assert_eq!(staged.len(), 2);
}

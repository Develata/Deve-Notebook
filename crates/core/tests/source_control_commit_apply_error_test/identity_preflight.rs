use deve_core::ledger::RepoManager;
use deve_core::ledger::range;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::staging;
use tempfile::tempdir;

#[test]
fn apply_external_changes_rejects_delete_target_when_doc_id_path_mismatches() {
    let dir = tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    let (doc_a, _ops) = repo
        .apply_file_structure_in_local_repo("default", "notes/a.md", None, "test")
        .expect("doc a");
    let (doc_b, _ops) = repo
        .apply_file_structure_in_local_repo("default", "notes/b.md", None, "test")
        .expect("doc b");
    assert_ne!(doc_a, doc_b);
    repo.commit_source_control_changes("baseline")
        .expect("commit baseline");
    repo.run_on_local_repo("default", |db| {
        staging::stage_pending_entry(
            db,
            &PendingFsEntry {
                path: "notes/b.md".into(),
                renamed_from: None,
                doc_id: Some(doc_a),
                change_type: ChangeStatus::Deleted,
                content_hash: pending_fs::content_hash(""),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed corrupted staged delete");
    let before_seq = repo
        .run_on_local_repo("default", range::get_max_seq)
        .expect("before seq");

    let err = repo
        .apply_external_changes()
        .expect_err("delete target mismatch must fail closed");

    let after_seq = repo
        .run_on_local_repo("default", range::get_max_seq)
        .expect("after seq");
    let staged = repo
        .run_on_local_repo("default", staging::list_staged_entries)
        .expect("staged retained");
    assert!(
        err.to_string().contains("delete target path mismatch"),
        "unexpected error: {}",
        err
    );
    assert_eq!(after_seq, before_seq);
    assert_eq!(staged.len(), 1);
}

#[test]
fn apply_external_changes_rejects_upsert_target_when_path_is_bound_to_another_doc() {
    let dir = tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    let (doc_a, _ops) = repo
        .apply_file_structure_in_local_repo("default", "notes/a.md", None, "test")
        .expect("doc a");
    let (doc_b, _ops) = repo
        .apply_file_structure_in_local_repo("default", "notes/b.md", None, "test")
        .expect("doc b");
    assert_ne!(doc_a, doc_b);
    repo.commit_source_control_changes("baseline")
        .expect("commit baseline");
    let disk_path = repo
        .local_repo_workspace_path("default", "notes/b.md")
        .expect("workspace path");
    std::fs::create_dir_all(disk_path.parent().expect("parent")).expect("create parent");
    std::fs::write(&disk_path, "corrupt").expect("write staged file");
    repo.run_on_local_repo("default", |db| {
        staging::stage_pending_entry(
            db,
            &PendingFsEntry {
                path: "notes/b.md".into(),
                renamed_from: None,
                doc_id: Some(doc_a),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("corrupt"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed corrupted staged upsert");
    let before_seq = repo
        .run_on_local_repo("default", range::get_max_seq)
        .expect("before seq");

    let err = repo
        .apply_external_changes()
        .expect_err("upsert target bound to another doc must fail closed");

    let after_seq = repo
        .run_on_local_repo("default", range::get_max_seq)
        .expect("after seq");
    let staged = repo
        .run_on_local_repo("default", staging::list_staged_entries)
        .expect("staged retained");
    assert!(
        err.to_string().contains("upsert target path mismatch"),
        "unexpected error: {}",
        err
    );
    assert_eq!(after_seq, before_seq);
    assert_eq!(staged.len(), 1);
}

#[test]
fn apply_external_changes_rejects_upsert_move_without_rename_evidence() {
    let dir = tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo("default", "notes/a.md", None, "test")
        .expect("doc a");
    repo.commit_source_control_changes("baseline")
        .expect("commit baseline");
    let disk_path = repo
        .local_repo_workspace_path("default", "notes/c.md")
        .expect("workspace path");
    std::fs::create_dir_all(disk_path.parent().expect("parent")).expect("create parent");
    std::fs::write(&disk_path, "corrupt").expect("write staged file");
    repo.run_on_local_repo("default", |db| {
        staging::stage_pending_entry(
            db,
            &PendingFsEntry {
                path: "notes/c.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("corrupt"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed corrupted staged move");
    let before_seq = repo
        .run_on_local_repo("default", range::get_max_seq)
        .expect("before seq");

    let err = repo
        .apply_external_changes()
        .expect_err("upsert move without rename evidence must fail closed");

    let after_seq = repo
        .run_on_local_repo("default", range::get_max_seq)
        .expect("after seq");
    let staged = repo
        .run_on_local_repo("default", staging::list_staged_entries)
        .expect("staged retained");
    assert!(
        err.to_string().contains("lacks rename evidence"),
        "unexpected error: {}",
        err
    );
    assert_eq!(after_seq, before_seq);
    assert_eq!(staged.len(), 1);
}

#[test]
fn apply_external_changes_rejects_docless_upsert_on_tracked_path() {
    let dir = tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo("default", "notes/a.md", None, "test")
        .expect("doc a");
    repo.commit_source_control_changes("baseline")
        .expect("commit baseline");
    let disk_path = repo
        .local_repo_workspace_path("default", "notes/a.md")
        .expect("workspace path");
    std::fs::create_dir_all(disk_path.parent().expect("parent")).expect("create parent");
    std::fs::write(&disk_path, "corrupt").expect("write staged file");
    repo.run_on_local_repo("default", |db| {
        staging::stage_pending_entry(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("corrupt"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed docless staged upsert");
    let before_seq = repo
        .run_on_local_repo("default", range::get_max_seq)
        .expect("before seq");

    let err = repo
        .apply_external_changes()
        .expect_err("docless upsert on tracked path must fail closed");

    let after_seq = repo
        .run_on_local_repo("default", range::get_max_seq)
        .expect("after seq");
    let staged = repo
        .run_on_local_repo("default", staging::list_staged_entries)
        .expect("staged retained");
    assert!(
        err.to_string()
            .contains("docless upsert target points at tracked path"),
        "unexpected error: {}",
        err
    );
    assert_eq!(after_seq, before_seq);
    assert_eq!(staged.len(), 1);
    assert_eq!(staged[0].1.doc_id, None);
    assert_eq!(
        repo.get_tracked_docid_in_local_repo("default", "notes/a.md")
            .expect("tracked doc"),
        Some(doc_id)
    );
}

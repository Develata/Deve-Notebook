use super::*;

#[test]
fn commit_staged_rejects_delete_target_when_doc_id_path_mismatches() {
    let dir = tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    let (doc_a, _ops) = repo
        .apply_file_structure_in_local_repo("default", "notes/a.md", None, "test")
        .expect("doc a");
    let (doc_b, _ops) = repo
        .apply_file_structure_in_local_repo("default", "notes/b.md", None, "test")
        .expect("doc b");
    assert_ne!(doc_a, doc_b);
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
        .commit_staged("delete corrupt")
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
fn commit_staged_rejects_upsert_target_when_path_is_bound_to_another_doc() {
    let dir = tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    let (doc_a, _ops) = repo
        .apply_file_structure_in_local_repo("default", "notes/a.md", None, "test")
        .expect("doc a");
    let (doc_b, _ops) = repo
        .apply_file_structure_in_local_repo("default", "notes/b.md", None, "test")
        .expect("doc b");
    assert_ne!(doc_a, doc_b);
    let disk_path = dir.path().join("vault/default/notes/b.md");
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
        .commit_staged("upsert corrupt")
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
fn commit_staged_rejects_upsert_move_without_rename_evidence() {
    let dir = tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo("default", "notes/a.md", None, "test")
        .expect("doc a");
    let disk_path = dir.path().join("vault/default/notes/c.md");
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
        .commit_staged("move corrupt")
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

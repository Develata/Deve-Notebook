use deve_core::ledger::RepoManager;
use deve_core::ledger::range;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::models::DocId;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::staging;
use tempfile::tempdir;

#[test]
fn apply_file_structure_fails_closed_when_legacy_path_binding_conflicts() {
    let dir = tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    let doc_id = DocId::new();
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        {
            let mut p2d = write.open_table(PATH_TO_DOCID)?;
            let mut d2p = write.open_table(DOCID_TO_PATH)?;
            p2d.insert("notes/legacy.md", doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), "notes/legacy.md")?;
        }
        write.commit()?;
        Ok::<_, anyhow::Error>(())
    })
    .expect("seed legacy path mapping");

    let err = repo
        .apply_file_structure_in_local_repo("default", "notes/legacy.md", None, "test")
        .expect_err("legacy path binding conflict must fail closed");
    assert!(
        err.to_string().contains("already bound"),
        "expected binding conflict, got: {}",
        err
    );
}

#[test]
fn apply_file_delete_structure_returns_none_when_only_legacy_path_mapping_exists() {
    let dir = tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    let doc_id = DocId::new();
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        {
            let mut p2d = write.open_table(PATH_TO_DOCID)?;
            let mut d2p = write.open_table(DOCID_TO_PATH)?;
            p2d.insert("notes/legacy-delete.md", doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), "notes/legacy-delete.md")?;
        }
        write.commit()?;
        Ok::<_, anyhow::Error>(())
    })
    .expect("seed legacy path mapping");

    let result = repo
        .apply_file_delete_structure_in_local_repo(
            "default",
            "notes/legacy-delete.md",
            None,
            "test",
        )
        .expect("legacy-only path is ignored");
    assert!(
        result.is_none(),
        "legacy-only path must not resolve for delete"
    );
}

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

#[test]
fn commit_staged_preflights_all_workspace_files_before_ledger_append() {
    let dir = tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    let existing_path = dir.path().join("vault/default/notes/a.md");
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
        .commit_staged("commit mixed")
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

use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::models::DocId;
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

#[test]
fn apply_file_structure_fails_closed_when_only_legacy_path_mapping_exists() {
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
        .expect_err("legacy-only path must fail closed");
    assert!(err
        .to_string()
        .contains("Tracked document projection missing for legacy-mapped path"));
}

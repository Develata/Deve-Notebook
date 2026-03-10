use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::models::DocId;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    (dir, repo)
}

#[test]
fn discard_pending_added_cleans_legacy_mapping() {
    let (dir, repo) = new_repo();
    let file = dir
        .path()
        .join("vault")
        .join("default")
        .join("notes/new.md");
    std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file, "temp").expect("write file");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/new.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("temp"),
                detected_at: 1,
                has_conflict: false,
            },
        )?;
        let doc_id = DocId::new();
        let write_txn = db.begin_write()?;
        {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;
            p2d.insert("notes/new.md", doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), "notes/new.md")?;
        }
        write_txn.commit()?;
        Ok(())
    })
    .expect("seed legacy mapping");

    repo.discard_pending("notes/new.md").expect("discard added");

    assert!(!file.exists());
    assert!(
        repo.get_docid("notes/new.md")
            .expect("lookup docid")
            .is_none()
    );
}

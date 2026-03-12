use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID, PATH_TO_NODEID};
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::changes;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    (dir, repo)
}

fn seed_file(repo: &RepoManager, path: &str, content: &str) -> DocId {
    let doc_id = repo
        .apply_file_structure_in_local_repo(repo.local_repo_name(), path, None, "test")
        .expect("create file");
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        PeerId::new("test"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: content.into(),
                },
                1,
                PeerId::new("test"),
                seq,
                None,
                None,
            )
        },
    )
    .expect("append content");
    doc_id
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

#[test]
fn workdir_diff_does_not_fallback_to_legacy_snapshot_path_index() {
    let (dir, repo) = new_repo();
    let doc_id = seed_file(&repo, "notes/a.md", "hello");
    let file = dir.path().join("vault").join("default").join("notes/a.md");
    std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file, "workspace hello").expect("write workspace");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        changes::save_snapshot(db, doc_id, "hello")?;
        let write_txn = db.begin_write()?;
        {
            let mut p2n = write_txn.open_table(PATH_TO_NODEID)?;
            p2n.remove("notes/a.md")?;
        }
        write_txn.commit()?;
        Ok(())
    })
    .expect("poison path lookup");

    let (old_content, new_content) = repo
        .workdir_diff_inputs_in_local_repo(repo.local_repo_name(), "notes/a.md")
        .expect("workdir diff");

    assert_eq!(old_content, "");
    assert_eq!(new_content, "workspace hello");
}

use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID, PATH_TO_NODEID};
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::changes;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos(dir.path().join("notes"));
    (dir, repo)
}

fn seed_file(repo: &RepoManager, path: &str, content: &str) -> DocId {
    let (doc_id, _ops) = repo
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
fn discard_pending_added_succeeds_on_legacy_only_projection() {
    let (_dir, repo) = new_repo();
    let file = repo
        .local_repo_workspace_path("default", "notes/new.md")
        .expect("workspace path");
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

    repo.discard_pending("notes/new.md")
        .expect("discard succeeds; legacy-only mapping is ignored");

    assert!(
        !file.exists(),
        "workspace file should be removed after discard"
    );
}

#[test]
fn workdir_diff_returns_empty_old_content_for_legacy_only_path() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_file(&repo, "notes/a.md", "hello");
    let file = repo
        .local_repo_workspace_path("default", "notes/a.md")
        .expect("workspace path");
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

    let (old, new) = repo
        .workdir_diff_inputs_in_local_repo(repo.local_repo_name(), "notes/a.md")
        .expect("diff succeeds; legacy path returns empty old content");
    assert!(
        old.is_empty(),
        "old content should be empty for poisoned node-first path"
    );
    assert_eq!(new, "workspace hello");
}

#[cfg(unix)]
#[test]
fn workdir_diff_fails_closed_on_unstatable_workspace_file() {
    let (_dir, repo) = new_repo();
    let _doc_id = seed_file(&repo, "notes/a.md", "hello");
    let notes_dir = repo
        .local_repo_workspace_path("default", "notes")
        .expect("workspace path");
    let file = notes_dir.join("a.md");
    std::fs::create_dir_all(&notes_dir).expect("mkdir");
    std::fs::write(&file, "workspace hello").expect("write workspace");

    let original = std::fs::metadata(&notes_dir)
        .expect("metadata")
        .permissions();
    let mut blocked = original.clone();
    blocked.set_mode(0o000);
    std::fs::set_permissions(&notes_dir, blocked).expect("chmod 000");

    let err = repo
        .workdir_diff_inputs_in_local_repo(repo.local_repo_name(), "notes/a.md")
        .expect_err("unstatable workspace file must fail closed");

    std::fs::set_permissions(&notes_dir, original).expect("restore perms");
    assert!(
        err.to_string().contains("Failed to stat workspace path")
            || err.to_string().contains("Permission denied")
    );
}

use deve_core::ledger::RepoManager;
use deve_core::models::{LedgerEntry, Op, PeerId};
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::sync::SyncManager;
use std::sync::Arc;
use tempfile::TempDir;

fn new_repo() -> (TempDir, Arc<RepoManager>) {
    let dir = TempDir::new().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init");
    repo.set_vault_root(dir.path().join("vault"));
    (dir, Arc::new(repo))
}

#[test]
fn discard_restore_marks_projection_write_for_watcher_ignore() -> anyhow::Result<()> {
    let (dir, repo) = new_repo();
    let doc_id = repo.apply_file_structure_in_local_repo(
        repo.local_repo_name(),
        "notes/a.md",
        None,
        "test",
    )?;
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        PeerId::new("test"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "ledger".into(),
                },
                1,
                PeerId::new("test"),
                seq,
                None,
                None,
            )
        },
    )?;
    let file_path = dir.path().join("vault/default/notes/a.md");
    std::fs::create_dir_all(file_path.parent().expect("parent"))?;
    std::fs::write(&file_path, "dirty")?;
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("dirty"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })?;
    let sync = SyncManager::new(repo, dir.path().join("vault"));
    sync.discard_pending_in_local_repo("default", "notes/a.md")?;
    assert_eq!(std::fs::read_to_string(file_path)?, "ledger");
    assert!(sync.should_ignore_fs_event("default/notes/a.md"));
    Ok(())
}

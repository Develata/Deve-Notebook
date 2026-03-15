use deve_core::ledger::RepoManager;
use deve_core::models::DocId;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use tempfile::tempdir;

#[test]
fn repo_discard_target_fails_closed_when_doc_id_does_not_match() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
    repo.set_vault_root(dir.path().join("vault"));
    let doc_id = repo.apply_file_structure_in_local_repo("default", "notes/live.md", None, "test")?;
    repo.run_on_local_repo("default", |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/live.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("dirty"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })?;

    let err = repo
        .discard_pending_target_in_local_repo(
            "default",
            &ScPathTarget {
                path: "notes/live.md".into(),
                doc_id: Some(DocId::new()),
            },
        )
        .expect_err("mismatched doc target must fail closed");

    assert!(err.to_string().contains("Path is not in pending_fs_ops"));
    Ok(())
}

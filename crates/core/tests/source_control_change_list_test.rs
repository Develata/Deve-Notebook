use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::staging;
use tempfile::tempdir;

mod common;

#[test]
fn list_changes_keeps_same_path_entries_for_different_docs() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let (repo, _repo_id) = common::init_cataloged_repo_with_depth(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )?;
    let doc_a = deve_core::models::DocId::new();
    let doc_b = deve_core::models::DocId::new();

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        staging::stage_pending_entry(
            db,
            &PendingFsEntry {
                path: "notes/reused.md".into(),
                renamed_from: None,
                doc_id: Some(doc_a),
                change_type: ChangeStatus::Deleted,
                content_hash: String::new(),
                detected_at: 1,
                has_conflict: false,
            },
        )?;
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/reused.md".into(),
                renamed_from: None,
                doc_id: Some(doc_b),
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("new"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })?;

    let changes = repo.list_changes_in_local_repo(repo.local_repo_name())?;
    assert_eq!(changes.len(), 2);
    assert!(changes.iter().any(|entry| {
        entry.path == "notes/reused.md"
            && entry.doc_id == Some(doc_a)
            && entry.status == ChangeStatus::Deleted
    }));
    assert!(changes.iter().any(|entry| {
        entry.path == "notes/reused.md"
            && entry.doc_id == Some(doc_b)
            && entry.status == ChangeStatus::Added
    }));
    Ok(())
}

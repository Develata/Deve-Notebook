use super::repair_doc_path;
use deve_core::models::DocId;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn repair_doc_path_rolls_back_workspace_and_mapping_on_pending_conflict() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let cataloged = crate::test_support::init_cataloged_repo(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )?;
    let repo_name = cataloged.repo.local_repo_name().to_string();
    let repo = Arc::new(cataloged.repo);

    let doc_id = DocId::new();
    let old_path = "default/notes/live.md";
    let new_path = "notes/live.md";
    repo.apply_file_structure_in_local_repo(&repo_name, old_path, Some(doc_id), "test")?;

    let old_abs = repo.local_repo_workspace_path(&repo_name, old_path)?;
    let new_abs = repo.local_repo_workspace_path(&repo_name, new_path)?;
    std::fs::create_dir_all(old_abs.parent().expect("old parent"))?;
    std::fs::write(&old_abs, "old")?;

    repo.run_on_local_repo(&repo_name, |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: old_path.into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Modified,
                content_hash: String::new(),
                detected_at: 1,
                has_conflict: false,
            },
        )?;
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: new_path.into(),
                renamed_from: None,
                doc_id: Some(DocId::new()),
                change_type: ChangeStatus::Modified,
                content_hash: String::new(),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })?;

    let err = repair_doc_path(&repo, &repo_name, doc_id, old_path, new_path)
        .expect_err("pending target conflict must fail closed and rollback");
    assert!(err.to_string().contains("Pending FS target already exists"));
    assert!(old_abs.exists());
    assert!(!new_abs.exists());

    let docs = repo.list_local_docs(Some(&repo_name))?;
    assert!(docs.contains(&(doc_id, old_path.to_string())));
    assert!(!docs.contains(&(doc_id, new_path.to_string())));

    let old_pending = repo.run_on_local_repo(&repo_name, |db| pending_fs::get(db, old_path))?;
    let new_pending = repo.run_on_local_repo(&repo_name, |db| pending_fs::get(db, new_path))?;
    assert_eq!(old_pending.and_then(|entry| entry.doc_id), Some(doc_id));
    assert!(new_pending.is_some());
    Ok(())
}

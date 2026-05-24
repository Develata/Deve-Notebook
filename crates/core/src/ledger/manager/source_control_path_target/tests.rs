use super::RepoManager;
use crate::source_control::ChangeStatus;
use crate::source_control::pending_fs::{self, PendingFsEntry};
use crate::source_control::staging;

#[test]
fn path_wrapper_preserves_doc_identity_from_exact_pending_entry() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None)?;
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))?;
    let (doc_id, _ops) = repo.apply_file_structure_in_local_repo(
        repo.local_repo_name(),
        "notes/a.md",
        None,
        "test",
    )?;
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "docs/a.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("hello"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })?;

    let target = repo.tracked_target_for_path_in_local_repo(repo.local_repo_name(), "docs/a.md")?;
    assert_eq!(target.doc_id, Some(doc_id));
    Ok(())
}

#[test]
fn path_wrapper_preserves_doc_identity_from_renamed_from_pending_entry() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None)?;
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))?;
    let (doc_id, _ops) = repo.apply_file_structure_in_local_repo(
        repo.local_repo_name(),
        "notes/a.md",
        None,
        "test",
    )?;
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "docs/a.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("hello"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })?;

    let target =
        repo.tracked_target_for_path_in_local_repo(repo.local_repo_name(), "notes/a.md")?;
    assert_eq!(target.path, "notes/a.md");
    assert_eq!(target.doc_id, Some(doc_id));
    Ok(())
}

#[test]
fn path_wrapper_fails_closed_when_old_path_is_reused() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None)?;
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))?;
    let doc_id = crate::models::DocId::new();
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "docs/a.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("hello"),
                detected_at: 1,
                has_conflict: false,
            },
        )?;
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("new"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })?;

    let err = repo
        .tracked_target_for_path_in_local_repo(repo.local_repo_name(), "notes/a.md")
        .expect_err("reused old path must fail closed");
    assert!(
        err.to_string()
            .contains("Ambiguous source control path target: notes/a.md")
    );
    Ok(())
}

#[test]
fn stage_wrapper_stages_renamed_entry_from_old_path() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None)?;
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))?;
    let (doc_id, _ops) = repo.apply_file_structure_in_local_repo(
        repo.local_repo_name(),
        "notes/a.md",
        None,
        "test",
    )?;
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "docs/a.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("hello"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })?;

    repo.stage_pending_in_local_repo(repo.local_repo_name(), "notes/a.md")?;
    let staged = repo
        .run_on_local_repo(repo.local_repo_name(), |db| -> anyhow::Result<_> {
            staging::get_staged(db, "docs/a.md")
        })?
        .expect("renamed entry staged");
    assert_eq!(staged.doc_id, Some(doc_id));
    assert_eq!(staged.renamed_from.as_deref(), Some("notes/a.md"));
    Ok(())
}

#[test]
fn path_wrapper_keeps_docless_exact_delete_path_only() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None)?;
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))?;
    let (_doc_id, _ops) = repo.apply_file_structure_in_local_repo(
        repo.local_repo_name(),
        "notes/a.md",
        None,
        "test",
    )?;
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Deleted,
                content_hash: String::new(),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })?;

    let target =
        repo.tracked_target_for_path_in_local_repo(repo.local_repo_name(), "notes/a.md")?;
    assert_eq!(target.path, "notes/a.md");
    assert_eq!(target.doc_id, None);
    Ok(())
}

#[test]
fn path_wrapper_promotes_docless_non_delete_to_tracked_identity() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None)?;
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))?;
    let (doc_id, _ops) = repo.apply_file_structure_in_local_repo(
        repo.local_repo_name(),
        "notes/a.md",
        None,
        "test",
    )?;
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("modified"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })?;

    let target =
        repo.tracked_target_for_path_in_local_repo(repo.local_repo_name(), "notes/a.md")?;
    assert_eq!(target.path, "notes/a.md");
    assert_eq!(target.doc_id, Some(doc_id));
    Ok(())
}

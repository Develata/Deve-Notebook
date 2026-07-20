use super::{
    move_pending, path_exists_for_repair, prune_empty_parents, rename_workspace_file,
    validate_workspace_rename_target,
};
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::models::DocId;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
#[cfg(unix)]
use std::io::ErrorKind;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use tempfile::tempdir;

#[cfg(unix)]
fn assert_permission_denied_by_workspace_ancestor(err: &anyhow::Error) {
    assert!(
        err.to_string()
            .contains("Failed to stat Projection Workspace ancestor while resolving"),
        "unexpected workspace containment diagnostic: {err:#}"
    );
    assert!(
        err.chain().any(|cause| {
            cause
                .downcast_ref::<std::io::Error>()
                .is_some_and(|io_err| io_err.kind() == ErrorKind::PermissionDenied)
        }),
        "workspace containment error must preserve PermissionDenied in its chain: {err:#}"
    );
}

#[test]
fn path_exists_for_repair_prefers_tracked_lookup() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let cataloged = crate::test_support::init_cataloged_repo(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )?;
    let repo_name = cataloged.repo.local_repo_name().to_string();
    let repo = Arc::new(cataloged.repo);
    let doc_id = DocId::new();
    assert!(!path_exists_for_repair(
        &repo,
        &repo_name,
        "notes/a.md",
        doc_id
    )?);
    Ok(())
}

#[test]
fn path_exists_for_repair_returns_false_for_legacy_only_path_mapping() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let cataloged = crate::test_support::init_cataloged_repo(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )?;
    let repo_name = cataloged.repo.local_repo_name().to_string();
    let repo = Arc::new(cataloged.repo);
    let doc_id = DocId::new();
    repo.run_on_local_repo(&repo_name, |db| {
        let write = db.begin_write()?;
        {
            let mut p2d = write.open_table(PATH_TO_DOCID)?;
            let mut d2p = write.open_table(DOCID_TO_PATH)?;
            p2d.insert("notes/legacy.md", doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), "notes/legacy.md")?;
        }
        write.commit()?;
        Ok(())
    })?;

    assert!(
        !path_exists_for_repair(&repo, &repo_name, "notes/legacy.md", DocId::new())?,
        "legacy-only path must not be treated as existing tracked doc"
    );
    Ok(())
}

#[test]
fn path_exists_for_repair_rejects_other_tracked_doc() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let cataloged = crate::test_support::init_cataloged_repo(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )?;
    let repo_name = cataloged.repo.local_repo_name().to_string();
    let repo = Arc::new(cataloged.repo);
    let live = DocId::new();
    repo.apply_file_structure_in_local_repo(&repo_name, "notes/live.md", Some(live), "test")?;

    assert!(path_exists_for_repair(
        &repo,
        &repo_name,
        "notes/live.md",
        DocId::new()
    )?);
    Ok(())
}

#[test]
fn move_pending_ignores_entries_without_matching_doc_id() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let cataloged = crate::test_support::init_cataloged_repo(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )?;
    let repo_name = cataloged.repo.local_repo_name().to_string();
    let repo = Arc::new(cataloged.repo);
    repo.run_on_local_repo(&repo_name, |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "default/notes/live.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Modified,
                content_hash: String::new(),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })?;

    move_pending(
        &repo,
        &repo_name,
        DocId::new(),
        "default/notes/live.md",
        "notes/live.md",
    )?;
    let old_exists = repo.run_on_local_repo(&repo_name, |db| {
        Ok(pending_fs::get(db, "default/notes/live.md")?.is_some())
    })?;
    assert!(old_exists);
    Ok(())
}

#[test]
fn prune_empty_parents_stops_when_start_dir_is_missing() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let root = dir.path().join("notes").join("default");
    let parent = root.join("notes");
    std::fs::create_dir_all(&parent)?;
    let missing = parent.join("missing");

    prune_empty_parents(root.clone(), Some(&missing));

    assert!(
        parent.exists(),
        "missing start dir must not cause parent pruning"
    );
    Ok(())
}

#[test]
fn validate_workspace_rename_target_fails_closed_when_target_exists() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let cataloged = crate::test_support::init_cataloged_repo(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )?;
    let repo_name = cataloged.repo.local_repo_name().to_string();
    let repo = cataloged.repo;
    let old_abs = repo.local_repo_workspace_path(&repo_name, "default/notes/live.md")?;
    let new_abs = repo.local_repo_workspace_path(&repo_name, "notes/live.md")?;
    std::fs::create_dir_all(old_abs.parent().expect("old parent"))?;
    std::fs::create_dir_all(new_abs.parent().expect("new parent"))?;
    std::fs::write(&old_abs, "old")?;
    std::fs::write(&new_abs, "new")?;

    let err = validate_workspace_rename_target(
        &repo,
        &repo_name,
        "default/notes/live.md",
        "notes/live.md",
    )
    .expect_err("conflicting workspace target must fail closed");
    assert!(
        err.to_string()
            .contains("Repair target path already exists")
    );
    Ok(())
}

#[test]
fn rename_workspace_file_fails_closed_when_target_exists() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let cataloged = crate::test_support::init_cataloged_repo(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )?;
    let repo_name = cataloged.repo.local_repo_name().to_string();
    let repo = cataloged.repo;
    let old_abs = repo.local_repo_workspace_path(&repo_name, "default/notes/live.md")?;
    let new_abs = repo.local_repo_workspace_path(&repo_name, "notes/live.md")?;
    std::fs::create_dir_all(old_abs.parent().expect("old parent"))?;
    std::fs::create_dir_all(new_abs.parent().expect("new parent"))?;
    std::fs::write(&old_abs, "old")?;
    std::fs::write(&new_abs, "new")?;

    let err = rename_workspace_file(&repo, &repo_name, "default/notes/live.md", "notes/live.md")
        .expect_err("conflicting workspace target must fail closed");
    assert!(
        err.to_string()
            .contains("Repair target path already exists")
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn validate_workspace_rename_target_fails_closed_when_source_is_unstatable() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let cataloged = crate::test_support::init_cataloged_repo(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )?;
    let repo_name = cataloged.repo.local_repo_name().to_string();
    let repo = cataloged.repo;
    let notes_dir = repo.local_repo_workspace_path(&repo_name, "default/notes")?;
    let old_abs = repo.local_repo_workspace_path(&repo_name, "default/notes/live.md")?;
    std::fs::create_dir_all(&notes_dir)?;
    std::fs::write(&old_abs, "old")?;
    let original = std::fs::metadata(&notes_dir)?.permissions();
    let mut blocked = original.clone();
    blocked.set_mode(0o000);
    std::fs::set_permissions(&notes_dir, blocked)?;

    let err = validate_workspace_rename_target(
        &repo,
        &repo_name,
        "default/notes/live.md",
        "notes/live.md",
    )
    .expect_err("unstatable source must fail closed");

    std::fs::set_permissions(&notes_dir, original)?;
    assert_permission_denied_by_workspace_ancestor(&err);
    Ok(())
}

#[cfg(unix)]
#[test]
fn rename_workspace_file_fails_closed_when_source_is_unstatable() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let cataloged = crate::test_support::init_cataloged_repo(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )?;
    let repo_name = cataloged.repo.local_repo_name().to_string();
    let repo = cataloged.repo;
    let notes_dir = repo.local_repo_workspace_path(&repo_name, "default/notes")?;
    let old_abs = repo.local_repo_workspace_path(&repo_name, "default/notes/live.md")?;
    std::fs::create_dir_all(&notes_dir)?;
    std::fs::write(&old_abs, "old")?;
    let original = std::fs::metadata(&notes_dir)?.permissions();
    let mut blocked = original.clone();
    blocked.set_mode(0o000);
    std::fs::set_permissions(&notes_dir, blocked)?;

    let err = rename_workspace_file(&repo, &repo_name, "default/notes/live.md", "notes/live.md")
        .expect_err("unstatable source must fail closed");

    std::fs::set_permissions(&notes_dir, original)?;
    assert_permission_denied_by_workspace_ancestor(&err);
    Ok(())
}

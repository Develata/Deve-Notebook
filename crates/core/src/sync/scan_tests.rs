//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::{repo_relative_path, scan_local_repo_at_root};
use crate::source_control::pending_fs;
use crate::vfs::Vfs;
use std::path::Path;
use std::sync::Arc;

#[test]
fn repo_relative_path_fails_closed_when_path_escapes_repo_root() {
    let err = repo_relative_path(
        "default",
        Path::new("/projection/default"),
        Path::new("/tmp/a.md"),
    )
    .expect_err("escaped path must fail closed");
    assert!(err.to_string().contains("path escaped repo root"));
}

#[test]
fn watcher_startup_root_revalidation_does_not_create_changed_locator_root() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let old_base = dir.path().join("old");
    let new_base = dir.path().join("new");
    std::fs::create_dir_all(&old_base)?;
    std::fs::create_dir_all(&new_base)?;
    let (repo, repo_id) =
        crate::test_support::init_cataloged_repo(&dir.path().join("ledger"), &old_base)?;
    let name = repo_id.to_string();
    let old_root = repo.local_repo_workspace_root(&name)?;
    let old_root = std::fs::canonicalize(old_root)?;
    let vfs = Vfs::new(&old_root);

    repo.set_projection_base_for_local_repo(&name, &new_base)?;
    let changed_root = repo.local_repo_workspace_root(&name)?;
    assert!(!changed_root.exists());
    let repo = Arc::new(repo);

    scan_local_repo_at_root(&repo, &vfs, &name, &old_root)
        .expect_err("locator drift must fail startup root revalidation");
    assert!(
        !changed_root.exists(),
        "revalidation must not create a workspace at the changed locator"
    );
    Ok(())
}

#[test]
fn watcher_startup_scan_file_never_reresolves_the_locator() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let old_base = dir.path().join("old");
    let new_base = dir.path().join("new");
    std::fs::create_dir_all(&old_base)?;
    std::fs::create_dir_all(&new_base)?;
    let (repo, repo_id) =
        crate::test_support::init_cataloged_repo(&dir.path().join("ledger"), &old_base)?;
    let name = repo_id.to_string();
    let old_root = repo.local_repo_workspace_root(&name)?;
    let old_file = old_root.join("note.md");
    std::fs::write(&old_file, "old-root-content")?;
    let vfs = Vfs::new(&old_root);

    repo.set_projection_base_for_local_repo(&name, &new_base)?;
    let new_root = repo.local_repo_workspace_root(&name)?;
    std::fs::create_dir_all(&new_root)?;
    std::fs::write(new_root.join("note.md"), "new-root-content")?;
    let repo = Arc::new(repo);

    super::super::scan_file::scan_disk_file(&repo, &vfs, &name, "note.md", &old_file)?;
    let pending = repo
        .run_on_local_repo(&name, |db| pending_fs::get(db, "note.md"))?
        .expect("scan must record the walked file");
    assert_eq!(
        pending.content_hash,
        pending_fs::content_hash("old-root-content")
    );
    assert_ne!(
        pending.content_hash,
        pending_fs::content_hash("new-root-content")
    );
    Ok(())
}

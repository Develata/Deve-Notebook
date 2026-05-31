#[cfg(unix)]
use super::{
    is_ledger_managed_write_target, project_relative_path, resolve_capability_read_target,
    resolve_capability_write_target,
};
use super::{is_ledger_managed_write_target_for, managed_note_target_parts_for};
#[cfg(unix)]
use crate::plugin::manifest::Capability;
#[cfg(unix)]
use std::path::Path;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::symlink;

fn is_managed_for(
    repo: &crate::ledger::RepoManager,
    path: &std::path::Path,
) -> anyhow::Result<bool> {
    is_ledger_managed_write_target_for(repo, path).map_err(anyhow::Error::msg)
}

fn note_target_parts_for(
    repo: &crate::ledger::RepoManager,
    path: &std::path::Path,
) -> anyhow::Result<Option<(String, String)>> {
    managed_note_target_parts_for(repo, path).map_err(anyhow::Error::msg)
}

#[cfg(unix)]
#[test]
fn ledger_managed_detection_fails_closed_through_symlink() {
    let dir = tempdir().expect("tempdir");
    let cwd = std::env::current_dir().expect("cwd");
    let ledger = dir.path().join("ledger/local");
    std::fs::create_dir_all(&ledger).expect("mkdir");
    let target = ledger.join("wiki.redb");
    std::fs::write(&target, "ledger").expect("write");
    let alias_dir = dir.path().join("tmp");
    std::fs::create_dir_all(&alias_dir).expect("mkdir alias");
    let alias = alias_dir.join("alias.md");
    symlink(&target, &alias).expect("symlink");

    std::env::set_current_dir(dir.path()).expect("set cwd");
    let detected = is_ledger_managed_write_target(Path::new("tmp/alias.md"))
        .expect("managed detection should succeed");
    std::env::set_current_dir(cwd).expect("restore cwd");

    assert!(detected);
}

#[cfg(unix)]
#[test]
fn project_relative_path_uses_canonical_target_location() {
    let dir = tempdir().expect("tempdir");
    let workspace = dir.path().join("workspace/default/notes");
    std::fs::create_dir_all(&workspace).expect("mkdir");
    let target = workspace.join("a.md");
    std::fs::write(&target, "hello").expect("write");
    let alias_dir = dir.path().join("tmp");
    std::fs::create_dir_all(&alias_dir).expect("mkdir alias");
    let alias = alias_dir.join("alias.md");
    symlink(&target, &alias).expect("symlink");

    let rel = project_relative_path(dir.path(), Path::new("tmp/alias.md"))
        .expect("canonical relative path")
        .expect("inside project root");

    assert_eq!(rel, "workspace/default/notes/a.md");
}

#[cfg(unix)]
#[test]
fn capability_targets_return_canonical_symlink_destination() {
    let dir = tempdir().expect("tempdir");
    let allowed = dir.path().join("allowed");
    std::fs::create_dir_all(&allowed).expect("mkdir");
    let target = allowed.join("target.txt");
    std::fs::write(&target, "ok").expect("write");
    let alias = allowed.join("alias.txt");
    symlink(&target, &alias).expect("symlink");
    let caps = Capability {
        allow_fs_read: vec![allowed.clone()],
        allow_fs_write: vec![allowed],
        ..Default::default()
    };
    let canonical = std::fs::canonicalize(&target).expect("canonical target");

    assert_eq!(
        resolve_capability_read_target(&caps, &alias)
            .expect("read target")
            .expect("read allowed"),
        canonical
    );
    assert_eq!(
        resolve_capability_write_target(&caps, &alias)
            .expect("write target")
            .expect("write allowed"),
        canonical
    );
}

#[test]
fn custom_projection_workspace_paths_are_protected_plugin_targets() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("my-notebooks");
    let repo =
        crate::ledger::RepoManager::init(&ledger_dir, 8, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_local_repo("default", &projection_base)?;
    let workspace = repo.local_repo_workspace_root("default")?;
    std::fs::create_dir_all(workspace.join("notes"))?;
    std::fs::create_dir_all(workspace.join(".notegit"))?;
    std::fs::create_dir_all(workspace.join(".git/objects"))?;
    std::fs::write(workspace.join("notes/a.md"), "note")?;
    std::fs::write(workspace.join(".notegit/runtime.bin"), "runtime")?;
    std::fs::write(workspace.join(".git/objects/x"), "git")?;

    assert!(is_managed_for(&repo, &workspace.join("notes/a.md"))?);
    assert!(is_managed_for(
        &repo,
        &workspace.join(".notegit/runtime.bin")
    )?);
    assert!(is_managed_for(&repo, &workspace.join(".git/objects/x"))?);
    assert!(!is_managed_for(&repo, &workspace.join(".gitignore"))?);
    Ok(())
}

#[test]
fn projection_base_sibling_markdown_is_not_a_plugin_managed_target() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("my-notebooks");
    let repo =
        crate::ledger::RepoManager::init(&ledger_dir, 8, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_local_repo("default", &projection_base)?;
    std::fs::create_dir_all(repo.local_repo_workspace_root("default")?)?;
    std::fs::write(projection_base.join("a.md"), "sibling")?;

    assert!(!is_managed_for(&repo, &projection_base.join("a.md"))?);
    Ok(())
}

#[test]
fn custom_projection_workspace_note_target_resolves_repo_scope() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("my-notebooks");
    let repo =
        crate::ledger::RepoManager::init(&ledger_dir, 8, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_local_repo("default", &projection_base)?;
    let workspace = repo.local_repo_workspace_root("default")?;
    std::fs::create_dir_all(workspace.join("notes"))?;
    std::fs::write(workspace.join("notes/a.md"), "note")?;
    std::fs::write(projection_base.join("a.md"), "sibling")?;

    assert_eq!(
        note_target_parts_for(&repo, &workspace.join("notes/a.md"))?,
        Some(("default".into(), "notes/a.md".into()))
    );
    assert_eq!(
        note_target_parts_for(&repo, &projection_base.join("a.md"))?,
        None
    );
    Ok(())
}

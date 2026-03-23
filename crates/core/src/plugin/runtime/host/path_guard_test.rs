use super::{is_ledger_managed_write_target, project_relative_path};
use std::path::Path;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[cfg(unix)]
#[test]
fn ledger_managed_detection_fails_closed_through_symlink() {
    let dir = tempdir().expect("tempdir");
    let cwd = std::env::current_dir().expect("cwd");
    let vault = dir.path().join("vault/default/notes");
    std::fs::create_dir_all(&vault).expect("mkdir");
    let target = vault.join("a.md");
    std::fs::write(&target, "hello").expect("write");
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
    let vault = dir.path().join("vault/default/notes");
    std::fs::create_dir_all(&vault).expect("mkdir");
    let target = vault.join("a.md");
    std::fs::write(&target, "hello").expect("write");
    let alias_dir = dir.path().join("tmp");
    std::fs::create_dir_all(&alias_dir).expect("mkdir alias");
    let alias = alias_dir.join("alias.md");
    symlink(&target, &alias).expect("symlink");

    let rel = project_relative_path(dir.path(), Path::new("tmp/alias.md"))
        .expect("canonical relative path")
        .expect("inside project root");

    assert_eq!(rel, "vault/default/notes/a.md");
}

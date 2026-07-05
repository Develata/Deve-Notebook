//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::write_pull_files;
use deve_core::remote_projection::RemoteProjectionFile;
use std::fs;

#[test]
fn write_pull_files_overwrites_existing_file_without_temp_artifacts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().join("workspace");
    fs::create_dir(&workspace).expect("workspace");
    fs::write(workspace.join("a.md"), "old").expect("old");
    let files = vec![RemoteProjectionFile::new("a.md", b"new").expect("file")];

    let applied = write_pull_files(&workspace, &files).expect("write");
    applied.commit();

    assert_eq!(
        fs::read_to_string(workspace.join("a.md")).expect("content"),
        "new"
    );
    let leftovers = fs::read_dir(dir.path())
        .expect("parent")
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with(".deve-projection-pull-"))
        .collect::<Vec<_>>();
    assert!(leftovers.is_empty(), "leftover staging dirs: {leftovers:?}");
}

#[test]
fn write_pull_files_rolls_back_when_not_committed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().join("workspace");
    fs::create_dir(&workspace).expect("workspace");
    fs::write(workspace.join("existing.md"), "old").expect("old");
    let files = vec![
        RemoteProjectionFile::new("existing.md", b"new").expect("existing"),
        RemoteProjectionFile::new("nested/new.md", b"added").expect("new"),
    ];

    let applied = write_pull_files(&workspace, &files).expect("write");
    assert_eq!(
        fs::read_to_string(workspace.join("existing.md")).expect("content"),
        "new"
    );
    assert_eq!(
        fs::read_to_string(workspace.join("nested").join("new.md")).expect("new file"),
        "added"
    );

    drop(applied);

    assert_eq!(
        fs::read_to_string(workspace.join("existing.md")).expect("content"),
        "old"
    );
    assert!(!workspace.join("nested").join("new.md").exists());
    assert!(!workspace.join("nested").exists());
}

#[test]
fn write_pull_files_explicit_rollback_reports_success() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().join("workspace");
    fs::create_dir(&workspace).expect("workspace");
    fs::write(workspace.join("a.md"), "old").expect("old");
    let files = vec![RemoteProjectionFile::new("a.md", b"new").expect("file")];

    let applied = write_pull_files(&workspace, &files).expect("write");
    applied.rollback_after_failed_scan().expect("rollback");

    assert_eq!(
        fs::read_to_string(workspace.join("a.md")).expect("content"),
        "old"
    );
}

#[test]
fn write_pull_files_rejects_blocked_parent_without_partial_apply() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().join("workspace");
    fs::create_dir(&workspace).expect("workspace");
    fs::write(workspace.join("a-good.md"), "old").expect("old");
    fs::write(workspace.join("blocked"), "not a directory").expect("blocked");
    let files = vec![
        RemoteProjectionFile::new("a-good.md", b"new").expect("good"),
        RemoteProjectionFile::new("blocked/new.md", b"blocked").expect("blocked"),
    ];

    let err = write_pull_files(&workspace, &files).expect_err("blocked parent");

    assert!(
        err.to_string()
            .contains("projection parent is not a directory")
    );
    assert_eq!(
        fs::read_to_string(workspace.join("a-good.md")).expect("good"),
        "old"
    );
    assert!(!workspace.join("blocked").join("new.md").exists());
}

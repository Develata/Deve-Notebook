use super::{
    BackupBindingStoreError, backup_binding_store_path_for, list_backup_binding_records,
    persist_backup_branch_binding, remove_backup_branch_binding,
};
use crate::backup::{
    BackupBindingAccess, BackupBranchBinding, BackupBranchBindingInput, BackupLocator,
    plan_backup_branch_binding,
};

fn repo_id(value: u128) -> uuid::Uuid {
    uuid::Uuid::from_u128(value)
}

fn binding(
    repo_id: uuid::Uuid,
    writer_identity: &str,
    access: BackupBindingAccess,
) -> BackupBranchBinding {
    let locator = BackupLocator::parse("s3://bucket-name/deve/").unwrap();
    let branch = locator.branch_locator(writer_identity).unwrap();
    plan_backup_branch_binding(BackupBranchBindingInput {
        repo_id,
        branch_name: "main".into(),
        writer_identity: branch.writer_identity,
        local_writer_identity: writer_identity.into(),
        branch_path: branch.branch_path,
        requested_access: access,
    })
    .unwrap()
}

#[test]
fn backup_binding_store_persists_secret_free_host_local_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let locator = BackupLocator::parse("s3://bucket-name/deve/").unwrap();
    let binding = binding(repo_id(99), "writer-1", BackupBindingAccess::Writable);

    persist_backup_branch_binding(dir.path(), locator.clone(), binding.clone()).unwrap();

    let path = backup_binding_store_path_for(dir.path());
    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("backup-bindings.toml")
    );
    let content = std::fs::read_to_string(path).unwrap();
    assert!(content.contains("bucket-name"));
    assert!(!content.contains("credential"));
    assert!(!content.contains("key_ref"));

    let records = list_backup_binding_records(dir.path()).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].locator, locator);
    assert_eq!(records[0].binding, binding);
}

#[test]
fn backup_binding_store_replaces_same_branch_writer_binding() {
    let dir = tempfile::tempdir().unwrap();
    let locator = BackupLocator::parse("s3://bucket-name/deve/").unwrap();
    let binding = binding(repo_id(99), "writer-1", BackupBindingAccess::Writable);

    persist_backup_branch_binding(dir.path(), locator.clone(), binding.clone()).unwrap();
    persist_backup_branch_binding(dir.path(), locator, binding).unwrap();

    assert_eq!(list_backup_binding_records(dir.path()).unwrap().len(), 1);
}

#[test]
fn backup_binding_store_rejects_duplicate_writable_branch() {
    let dir = tempfile::tempdir().unwrap();
    let locator = BackupLocator::parse("s3://bucket-name/deve/").unwrap();
    persist_backup_branch_binding(
        dir.path(),
        locator.clone(),
        binding(repo_id(99), "writer-1", BackupBindingAccess::Writable),
    )
    .unwrap();

    let err = persist_backup_branch_binding(
        dir.path(),
        locator,
        binding(repo_id(99), "writer-2", BackupBindingAccess::Writable),
    )
    .expect_err("second writable branch binding must fail closed");

    assert!(matches!(
        err,
        BackupBindingStoreError::Binding(
            crate::backup::BackupBindingError::DuplicateWritableBranch
        )
    ));
}

#[test]
fn backup_binding_store_removes_existing_binding_only() {
    let dir = tempfile::tempdir().unwrap();
    let locator = BackupLocator::parse("s3://bucket-name/deve/").unwrap();
    let binding = binding(repo_id(99), "writer-1", BackupBindingAccess::Writable);

    let err = remove_backup_branch_binding(dir.path(), &locator, &binding)
        .expect_err("missing binding must fail closed");
    assert!(matches!(err, BackupBindingStoreError::MissingBinding));

    persist_backup_branch_binding(dir.path(), locator.clone(), binding.clone()).unwrap();
    let removed = remove_backup_branch_binding(dir.path(), &locator, &binding).unwrap();

    assert_eq!(removed.binding, binding);
    assert!(list_backup_binding_records(dir.path()).unwrap().is_empty());
}

#[test]
fn backup_binding_store_rejects_cross_repo_same_writable_physical_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let locator = BackupLocator::parse("s3://bucket-name/deve/").unwrap();
    persist_backup_branch_binding(
        dir.path(),
        locator.clone(),
        binding(repo_id(99), "writer-1", BackupBindingAccess::Writable),
    )
    .unwrap();

    let err = persist_backup_branch_binding(
        dir.path(),
        locator,
        binding(repo_id(100), "writer-1", BackupBindingAccess::Writable),
    )
    .expect_err("same physical backup prefix must fail closed across repos");

    assert!(matches!(
        err,
        BackupBindingStoreError::DuplicateWritablePhysicalPath
    ));
}

#[test]
fn backup_binding_store_rejects_non_regular_store_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = backup_binding_store_path_for(dir.path());
    std::fs::create_dir_all(&path).expect("directory at binding store path");

    let err = list_backup_binding_records(dir.path()).expect_err("directory must fail closed");

    assert!(matches!(
        err,
        BackupBindingStoreError::NonRegularStorePath(_)
    ));
}

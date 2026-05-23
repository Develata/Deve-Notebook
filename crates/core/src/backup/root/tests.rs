use super::{
    BACKUP_ROOT_FORMAT_VERSION, BackupProviderKind, BackupRootError, BackupRootInput,
    plan_backup_root,
};
use crate::backup::BackupLocator;

fn input() -> BackupRootInput {
    let repo_id = uuid::Uuid::from_u128(42);
    BackupRootInput {
        repo_locator: BackupLocator::parse("s3://bucket-name/deve/").unwrap(),
        expected_repo_id: repo_id,
        manifest_repo_id: repo_id,
        format_version: BACKUP_ROOT_FORMAT_VERSION,
        provider_kind: BackupProviderKind::S3,
    }
}

#[test]
fn plans_backup_root_from_matching_locator_and_manifest() {
    let root = plan_backup_root(input()).unwrap();

    assert_eq!(root.repo_id, uuid::Uuid::from_u128(42));
    assert_eq!(root.format_version, BACKUP_ROOT_FORMAT_VERSION);
    assert_eq!(root.provider_kind, BackupProviderKind::S3);
    assert_eq!(root.repo_locator.repo_root_path, "deve");
}

#[test]
fn rejects_manifest_repo_id_mismatch() {
    let mut input = input();
    input.manifest_repo_id = uuid::Uuid::from_u128(7);

    assert_eq!(
        plan_backup_root(input),
        Err(BackupRootError::RepoIdMismatch)
    );
}

#[test]
fn rejects_provider_kind_mismatch() {
    let mut input = input();
    input.provider_kind = BackupProviderKind::WebDavHttps;

    assert_eq!(
        plan_backup_root(input),
        Err(BackupRootError::ProviderKindMismatch)
    );
}

#[test]
fn rejects_unsupported_format_version() {
    let mut input = input();
    input.format_version = BACKUP_ROOT_FORMAT_VERSION + 1;

    assert_eq!(
        plan_backup_root(input),
        Err(BackupRootError::UnsupportedFormatVersion(
            BACKUP_ROOT_FORMAT_VERSION + 1
        ))
    );
}

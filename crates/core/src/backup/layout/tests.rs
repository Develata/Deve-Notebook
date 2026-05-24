use super::{
    BackupRemoteLayoutDiagnosticKind, BackupRemoteLayoutError, BackupRemoteLayoutInput,
    BackupRemoteObject, BackupTransportMetadata, inspect_backup_remote_layout,
};
use crate::backup::BackupLocator;

fn branch() -> crate::backup::BranchBackupLocator {
    BackupLocator::parse("s3://bucket-name/deve/")
        .unwrap()
        .branch_locator("writer-1")
        .unwrap()
}

fn object(path: &str) -> BackupRemoteObject {
    BackupRemoteObject {
        path: path.into(),
        metadata: None,
    }
}

#[test]
fn reports_healthy_layout_with_expected_manifests_and_pack() {
    let branch = branch();
    let report = inspect_backup_remote_layout(BackupRemoteLayoutInput {
        expected_pack_object_paths: vec!["deve/branches/writer-1/packs/000001.pack.enc".into()],
        objects: vec![
            object("deve/repo.manifest.enc"),
            object("deve/branches/writer-1/branch.manifest.enc"),
            object("deve/branches/writer-1/packs/000001.pack.enc"),
        ],
        branch,
    })
    .unwrap();

    assert!(report.is_healthy());
    assert_eq!(report.repo_manifest_path, "deve/repo.manifest.enc");
    assert_eq!(
        report.branch_manifest_path,
        "deve/branches/writer-1/branch.manifest.enc"
    );
    assert_eq!(report.pack_prefix, "deve/branches/writer-1/packs");
}

#[test]
fn provider_metadata_is_diagnostic_only() {
    let branch = branch();
    let report = inspect_backup_remote_layout(BackupRemoteLayoutInput {
        expected_pack_object_paths: vec!["deve/branches/writer-1/packs/000001.pack.enc".into()],
        objects: vec![
            object("deve/repo.manifest.enc"),
            object("deve/branches/writer-1/branch.manifest.enc"),
            BackupRemoteObject {
                path: "deve/branches/writer-1/packs/000001.pack.enc".into(),
                metadata: Some(BackupTransportMetadata {
                    etag: Some("etag-1".into()),
                    version: Some("v1".into()),
                    mtime_unix_ms: Some(1_710_000_000_000),
                    object_key: Some("provider-key".into()),
                }),
            },
        ],
        branch,
    })
    .unwrap();

    assert!(report.is_healthy());
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(
        report.diagnostics[0].kind,
        BackupRemoteLayoutDiagnosticKind::ProviderMetadataObserved
    );
}

#[test]
fn reports_missing_manifest_and_pack_as_structured_drift() {
    let branch = branch();
    let report = inspect_backup_remote_layout(BackupRemoteLayoutInput {
        expected_pack_object_paths: vec!["deve/branches/writer-1/packs/000001.pack.enc".into()],
        objects: vec![object("deve/repo.manifest.enc")],
        branch,
    })
    .unwrap();

    assert!(!report.is_healthy());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == BackupRemoteLayoutDiagnosticKind::MissingBranchManifest
    }));
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == BackupRemoteLayoutDiagnosticKind::MissingPack)
    );
}

#[test]
fn reports_unexpected_and_unsafe_paths_without_rebinding() {
    let branch = branch();
    let report = inspect_backup_remote_layout(BackupRemoteLayoutInput {
        expected_pack_object_paths: vec!["deve/branches/writer-1/packs/000001.pack.enc".into()],
        objects: vec![
            object("deve/repo.manifest.enc"),
            object("deve/branches/writer-1/branch.manifest.enc"),
            object("deve/branches/writer-1/packs/000001.pack.enc"),
            object("deve/branches/writer-2/packs/000001.pack.enc"),
            object("deve\\branches\\writer-1\\packs\\bad.pack.enc"),
        ],
        branch,
    })
    .unwrap();

    assert!(!report.is_healthy());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == BackupRemoteLayoutDiagnosticKind::PackOutsideBranchPrefix
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == BackupRemoteLayoutDiagnosticKind::UnsafeRemoteObjectPath
    }));
}

#[test]
fn rejects_expected_pack_path_outside_branch_prefix() {
    let branch = branch();
    let err = inspect_backup_remote_layout(BackupRemoteLayoutInput {
        expected_pack_object_paths: vec!["deve/branches/writer-2/packs/000001.pack.enc".into()],
        objects: Vec::new(),
        branch,
    })
    .unwrap_err();

    assert_eq!(
        err,
        BackupRemoteLayoutError::ExpectedPackOutsideBranchPrefix
    );
}

#[test]
fn rejects_duplicate_expected_pack_paths() {
    let branch = branch();
    let err = inspect_backup_remote_layout(BackupRemoteLayoutInput {
        expected_pack_object_paths: vec![
            "deve/branches/writer-1/packs/000001.pack.enc".into(),
            "deve/branches/writer-1/packs/000001.pack.enc".into(),
        ],
        objects: Vec::new(),
        branch,
    })
    .unwrap_err();

    assert_eq!(err, BackupRemoteLayoutError::DuplicateExpectedPackPath);
}

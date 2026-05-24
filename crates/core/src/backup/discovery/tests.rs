use super::{
    BackupBranchDiscoveryDiagnosticKind, BackupBranchDiscoveryInput, discover_backup_branches,
};
use crate::backup::{BackupLocator, BackupRemoteObject, BackupTransportMetadata};

fn locator() -> BackupLocator {
    BackupLocator::parse("s3://bucket-name/deve/").unwrap()
}

fn object(path: &str) -> BackupRemoteObject {
    BackupRemoteObject {
        path: path.into(),
        metadata: None,
    }
}

#[test]
fn discovers_branch_manifests_without_treating_packs_as_authority() {
    let report = discover_backup_branches(BackupBranchDiscoveryInput {
        repo_locator: locator(),
        objects: vec![
            object("deve/repo.manifest.enc"),
            object("deve/branches/writer-b/branch.manifest.enc"),
            object("deve/branches/writer-b/packs/000001.pack.enc"),
            object("deve/branches/writer-a/branch.manifest.enc"),
        ],
    });

    assert!(report.is_healthy());
    assert_eq!(report.repo_manifest_path, "deve/repo.manifest.enc");
    assert_eq!(report.branches.len(), 2);
    assert_eq!(report.branches[0].writer_identity, "writer-a");
    assert_eq!(
        report.branches[0].branch_manifest_path,
        "deve/branches/writer-a/branch.manifest.enc"
    );
    assert_eq!(
        report.branches[0].pack_prefix,
        "deve/branches/writer-a/packs"
    );
    assert_eq!(report.branches[1].writer_identity, "writer-b");
}

#[test]
fn provider_metadata_is_diagnostic_only_for_discovery() {
    let report = discover_backup_branches(BackupBranchDiscoveryInput {
        repo_locator: locator(),
        objects: vec![
            object("deve/repo.manifest.enc"),
            BackupRemoteObject {
                path: "deve/branches/writer-1/branch.manifest.enc".into(),
                metadata: Some(BackupTransportMetadata {
                    etag: Some("etag-1".into()),
                    version: Some("version-1".into()),
                    mtime_unix_ms: Some(1_710_000_000_000),
                    object_key: Some("provider-key".into()),
                }),
            },
        ],
    });

    assert!(report.is_healthy());
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(
        report.diagnostics[0].kind,
        BackupBranchDiscoveryDiagnosticKind::ProviderMetadataObserved
    );
}

#[test]
fn reports_missing_repo_manifest_but_keeps_branch_candidates() {
    let report = discover_backup_branches(BackupBranchDiscoveryInput {
        repo_locator: locator(),
        objects: vec![object("deve/branches/writer-1/branch.manifest.enc")],
    });

    assert!(!report.is_healthy());
    assert_eq!(report.branches.len(), 1);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == BackupBranchDiscoveryDiagnosticKind::MissingRepoManifest
    }));
}

#[test]
fn rejects_unsafe_writer_and_object_paths_without_discovery() {
    let report = discover_backup_branches(BackupBranchDiscoveryInput {
        repo_locator: locator(),
        objects: vec![
            object("deve/repo.manifest.enc"),
            object("deve/branches/../branch.manifest.enc"),
            object("deve/branches/writer+1/branch.manifest.enc"),
        ],
    });

    assert!(report.branches.is_empty());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == BackupBranchDiscoveryDiagnosticKind::UnsafeRemoteObjectPath
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == BackupBranchDiscoveryDiagnosticKind::UnsafeWriterIdentity
    }));
}

#[test]
fn reports_duplicate_manifest_and_outside_repo_root() {
    let report = discover_backup_branches(BackupBranchDiscoveryInput {
        repo_locator: locator(),
        objects: vec![
            object("deve/repo.manifest.enc"),
            object("deve/branches/writer-1/branch.manifest.enc"),
            object("deve/branches/writer-1/branch.manifest.enc"),
            object("other/branches/writer-2/branch.manifest.enc"),
        ],
    });

    assert_eq!(report.branches.len(), 1);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == BackupBranchDiscoveryDiagnosticKind::DuplicateObjectPath
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == BackupBranchDiscoveryDiagnosticKind::DuplicateBranchManifest
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == BackupBranchDiscoveryDiagnosticKind::OutsideRepoRoot
    }));
}

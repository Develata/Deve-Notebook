use super::{inspect_lines, list_lines};

#[test]
fn backup_inspect_prints_sanitized_locator_components() {
    let lines = inspect_lines(
        "s3+https://r2.example.com/bucket-name/deve/",
        Some("writer-1"),
        None,
        None,
    )
    .expect("inspect");

    assert_eq!(lines[0], "backup_locator: provider=s3+https");
    assert!(
        lines
            .iter()
            .any(|line| line == "endpoint=https://r2.example.com")
    );
    assert!(lines.iter().any(|line| line == "namespace=bucket-name"));
    assert!(lines.iter().any(|line| line == "repo_root_path=deve"));
    assert!(
        lines
            .iter()
            .any(|line| line == "branch_path=deve/branches/writer-1")
    );
}

#[test]
fn backup_inspect_can_plan_provider_adapter_with_redacted_refs() {
    let lines = inspect_lines(
        "webdav+https://dav.example.com/notebooks/deve/",
        None,
        Some("env:DEVE_BACKUP_TOKEN"),
        Some("keyring:deve/default-backup-key"),
    )
    .expect("inspect adapter");

    assert!(
        lines
            .iter()
            .any(|line| line == "command=InspectBackupTarget")
    );
    assert!(lines.iter().any(|line| line == "effect=InspectOnly"));
    assert!(
        lines
            .iter()
            .any(|line| line == "adapter_provider=webdav+https")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "credential_ref=env:<redacted>")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "key_ref=keyring:<redacted>")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "provider_metadata_diagnostic_only=true")
    );
}

#[test]
fn backup_inspect_requires_credential_and_key_refs_together() {
    let err = inspect_lines(
        "s3://bucket-name/deve/",
        None,
        Some("env:DEVE_BACKUP_TOKEN"),
        None,
    )
    .expect_err("partial refs must fail closed");

    assert!(err.to_string().contains("requires both"));
}

#[test]
fn backup_inspect_rejects_locator_with_secret_material() {
    let err = inspect_lines(
        "webdav+https://user:pass@dav.example.com/deve/",
        None,
        None,
        None,
    )
    .expect_err("secret material should fail closed");

    assert!(err.to_string().contains("must not contain credentials"));
}

#[test]
fn backup_list_prints_branch_candidates_from_object_paths() {
    let objects = vec![
        "deve/repo.manifest.enc".to_string(),
        "deve/branches/writer-b/branch.manifest.enc".to_string(),
        "deve/branches/writer-a/branch.manifest.enc".to_string(),
        "deve/branches/writer-a/packs/000001.pack.enc".to_string(),
    ];
    let lines = list_lines("s3://bucket-name/deve/", &objects).expect("list");

    assert!(
        lines
            .iter()
            .any(|line| line == "command=ListBackupBranches")
    );
    assert!(lines.iter().any(|line| line == "effect=InspectOnly"));
    assert!(lines.iter().any(|line| line == "branch_count=2"));
    assert!(lines.iter().any(|line| {
        line == "branch writer=writer-a path=deve/branches/writer-a manifest=deve/branches/writer-a/branch.manifest.enc pack_prefix=deve/branches/writer-a/packs"
    }));
}

#[test]
fn backup_list_reports_discovery_diagnostics_without_binding() {
    let objects = vec!["deve/branches/writer+1/branch.manifest.enc".to_string()];
    let lines = list_lines("s3://bucket-name/deve/", &objects).expect("list");

    assert!(lines.iter().any(|line| line == "branch_count=0"));
    assert!(
        lines
            .iter()
            .any(|line| line.contains("diagnostic kind=MissingRepoManifest"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("diagnostic kind=UnsafeWriterIdentity"))
    );
}

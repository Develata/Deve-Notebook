use super::{BackupLocator, BackupProviderKind};

#[test]
fn parses_webdav_https_locator() {
    let locator = BackupLocator::parse("webdav+https://dav.example.com/notebooks/deve/").unwrap();

    assert_eq!(locator.provider, BackupProviderKind::WebDavHttps);
    assert_eq!(locator.endpoint.as_deref(), Some("https://dav.example.com"));
    assert_eq!(locator.namespace, "dav.example.com");
    assert_eq!(locator.repo_root_path, "notebooks/deve");
}

#[test]
fn parses_s3_locator() {
    let locator = BackupLocator::parse("s3://bucket-name/deve/").unwrap();

    assert_eq!(locator.provider, BackupProviderKind::S3);
    assert_eq!(locator.endpoint, None);
    assert_eq!(locator.namespace, "bucket-name");
    assert_eq!(locator.repo_root_path, "deve");
}

#[test]
fn parses_s3_compatible_https_locator() {
    let locator = BackupLocator::parse("s3+https://r2.example.com/bucket-name/deve/").unwrap();

    assert_eq!(locator.provider, BackupProviderKind::S3CompatibleHttps);
    assert_eq!(locator.endpoint.as_deref(), Some("https://r2.example.com"));
    assert_eq!(locator.namespace, "bucket-name");
    assert_eq!(locator.repo_root_path, "deve");
}

#[test]
fn derives_branch_paths_without_writing_any_state() {
    let locator = BackupLocator::parse("s3://bucket-name/deve/").unwrap();
    let branch = locator.branch_locator("writer-1").unwrap();

    assert_eq!(branch.branch_path, "deve/branches/writer-1");
    assert_eq!(
        branch.branch_manifest_path(),
        "deve/branches/writer-1/branch.manifest.enc"
    );
    assert_eq!(branch.pack_prefix(), "deve/branches/writer-1/packs");
}

#[test]
fn rejects_credentials_query_fragment_and_unsafe_paths() {
    for input in [
        "webdav+https://user:pass@dav.example.com/deve/",
        "webdav+https://dav.example.com/deve/?token=secret",
        "s3+https://r2.example.com/bucket/deve/#frag",
        "s3://bucket/../deve",
        "s3://bucket//deve",
        "s3://bucket/deve//branch",
        "s3://bucket/deve path",
    ] {
        assert!(
            BackupLocator::parse(input).is_err(),
            "locator should be rejected: {input}"
        );
    }
}

#[test]
fn rejects_unsafe_branch_writer_identity() {
    let locator = BackupLocator::parse("s3://bucket-name/deve/").unwrap();

    for writer in [
        "../writer",
        "writer/name",
        "writer:name",
        " writer",
        "",
        ".",
    ] {
        assert!(
            locator.branch_locator(writer).is_err(),
            "writer identity should be rejected: {writer:?}"
        );
    }
}

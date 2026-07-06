use super::super::credentials::{S3CredentialSource, S3RegionSource};
use super::super::provider::S3ProjectionProvider;
use super::super::push::S3ProjectionPushAdapter;
use super::super::url::{s3_file_url, s3_list_url};
use super::support::{RecordingS3Transport, now, test_credentials};
use crate::commands::projection_remote::collect::collect_markdown_projection_files;
use deve_core::remote_projection::{
    RemoteProjectionFile, RemoteProjectionProvider, RemoteProjectionProviderAdapter,
    RemoteProjectionPullRequest, RemoteProjectionPushRequest,
};
use reqwest::StatusCode;
use std::fs;

#[test]
fn s3_custom_https_endpoint_requires_explicit_credential_binding() {
    let err = s3_file_url(
        "s3+https://minio.example.com/bucket/notebooks/main",
        "unused-region",
        "notes/a.md",
    )
    .expect_err("custom endpoint must fail closed");

    assert!(err.to_string().contains("explicit credential binding"));
    assert!(err.to_string().contains("provider_io_ready=false"));

    let err = s3_list_url(
        "s3+https://minio.example.com/bucket/notebooks/main",
        "unused-region",
        None,
    )
    .expect_err("custom endpoint list must fail closed");
    assert!(err.to_string().contains("explicit credential binding"));
    assert!(err.to_string().contains("provider_io_ready=false"));
}

#[test]
fn s3_custom_https_endpoint_fails_before_workspace_file_read() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("a.md");
    fs::write(&file, "a").expect("seed");
    let files = collect_markdown_projection_files(dir.path()).expect("collect");
    fs::remove_file(&file).expect("remove after collect");
    let transport = RecordingS3Transport::new(StatusCode::OK);
    let mut provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);

    let err = provider
        .push_projection_files(
            RemoteProjectionProvider::S3,
            "s3+https://minio.example.com/bucket/notebooks/main",
            &files,
        )
        .expect_err("custom endpoint must fail before file read");

    assert!(err.to_string().contains("explicit credential binding"));
    assert!(err.to_string().contains("provider_io_ready=false"));
    assert_eq!(
        provider
            .transport
            .put_calls
            .lock()
            .expect("put calls")
            .len(),
        0
    );
}

#[test]
fn s3_custom_https_endpoint_direct_push_fails_before_credentials_resolve() {
    let transport = RecordingS3Transport::new(StatusCode::OK);
    let mut provider = S3ProjectionProvider {
        transport,
        credentials: S3CredentialSource::Fail("custom-endpoint-push"),
        region: S3RegionSource::Fail("custom-endpoint-push"),
        now,
    };
    let request = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::S3,
        "s3+https://minio.example.com/bucket/notebooks/main",
        vec![RemoteProjectionFile::new("notes/a.md", b"a").expect("file")],
    )
    .expect("request");

    let err = provider
        .push(request)
        .expect_err("custom endpoint must fail before credentials resolve");

    assert!(err.to_string().contains("explicit credential binding"));
    assert!(err.to_string().contains("provider_io_ready=false"));
    assert_eq!(
        provider
            .transport
            .put_calls
            .lock()
            .expect("put calls")
            .len(),
        0
    );
}

#[test]
fn s3_custom_https_endpoint_direct_pull_fails_before_credentials_resolve() {
    let transport = RecordingS3Transport::new(StatusCode::OK);
    let provider = S3ProjectionProvider {
        transport,
        credentials: S3CredentialSource::Fail("custom-endpoint-pull"),
        region: S3RegionSource::Fail("custom-endpoint-pull"),
        now,
    };
    let request = RemoteProjectionPullRequest::new(
        RemoteProjectionProvider::S3,
        "s3+https://minio.example.com/bucket/notebooks/main",
    )
    .expect("request");

    let err = provider
        .pull(request)
        .expect_err("custom endpoint must fail before credentials resolve");

    assert!(err.to_string().contains("explicit credential binding"));
    assert!(err.to_string().contains("provider_io_ready=false"));
    assert_eq!(
        provider
            .transport
            .get_calls
            .lock()
            .expect("get calls")
            .len(),
        0
    );
}

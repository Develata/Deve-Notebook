use super::super::credentials::{S3CredentialSource, S3RegionSource};
use super::super::profile::RemoteProjectionS3Profile;
use super::super::provider::S3ProjectionProvider;
use super::super::push::{S3ProjectionPushAdapter, push_request};
use super::super::url::{
    reject_custom_https_endpoint_without_binding, s3_file_url, s3_list_url, s3_locator_prefix,
};
use super::support::{EnvGuard, RecordingS3Transport, header, now, test_credentials};
use crate::remote_projection_transport::{
    RemoteSourceAcquisition, SourceAcquisitionRequest, TestCollectingSourceSink,
    TransportCapability, WorkspaceProjectionPushSource,
};
use deve_core::remote_projection::{
    RemoteProjectionFile, RemoteProjectionProvider, RemoteProjectionPushRequest,
};
use reqwest::StatusCode;
use std::fs;

#[test]
fn s3_custom_https_endpoint_requires_explicit_credential_binding() {
    let err = s3_file_url(
        "S3+HTTPS://minio.example.com/bucket/notebooks/main",
        "unused-region",
        "notes/a.md",
    )
    .expect_err("custom endpoint must fail closed");

    assert!(err.to_string().contains("explicit credential binding"));
    assert!(err.to_string().contains("provider_io_ready=false"));

    let err = s3_list_url(
        "S3+HTTPS://minio.example.com/bucket/notebooks/main",
        "unused-region",
        None,
    )
    .expect_err("custom endpoint list must fail closed");
    assert!(err.to_string().contains("explicit credential binding"));
    assert!(err.to_string().contains("provider_io_ready=false"));

    let err = s3_locator_prefix("S3+HTTPS://minio.example.com/bucket/notebooks/main")
        .expect_err("custom endpoint prefix must fail closed");
    assert!(err.to_string().contains("explicit credential binding"));
    assert!(err.to_string().contains("provider_io_ready=false"));

    let err = reject_custom_https_endpoint_without_binding(
        "S3+HTTPS://minio.example.com/bucket/notebooks/main",
    )
    .expect_err("custom endpoint guard must fail closed");
    assert!(err.to_string().contains("explicit credential binding"));
    assert!(err.to_string().contains("provider_io_ready=false"));
}

#[test]
fn s3_aws_locator_scheme_matching_is_case_insensitive() {
    let file_url = s3_file_url("S3://bucket/notebooks/main", "us-east-1", "notes/a.md")
        .expect("uppercase s3 file URL");
    assert_eq!(
        file_url.as_str(),
        "https://bucket.s3.us-east-1.amazonaws.com/notebooks/main/notes/a.md"
    );

    let list_url = s3_list_url("S3://bucket/notebooks/main", "us-east-1", None)
        .expect("uppercase s3 list URL");
    assert_eq!(
        list_url.as_str(),
        "https://bucket.s3.us-east-1.amazonaws.com/?list-type=2&prefix=notebooks%2Fmain%2F"
    );

    assert_eq!(
        s3_locator_prefix("S3://bucket/notebooks/main").expect("prefix"),
        "notebooks/main/"
    );
}

#[test]
fn s3_custom_https_endpoint_fails_before_workspace_file_read() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("a.md");
    fs::write(&file, "a").expect("seed");
    let source = WorkspaceProjectionPushSource::collect(dir.path()).expect("collect");
    fs::remove_file(&file).expect("remove after collect");
    let transport = RecordingS3Transport::new(StatusCode::OK);
    let mut provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);

    let err = provider
        .push_projection_files(
            RemoteProjectionProvider::S3,
            "s3+https://minio.example.com/bucket/notebooks/main",
            &source,
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
    let provider = S3ProjectionProvider {
        transport,
        credentials: S3CredentialSource::Fail("custom-endpoint-push"),
        region: S3RegionSource::Fail("custom-endpoint-push"),
        custom_profile: None,
        now,
    };
    let request = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::S3,
        "s3+https://minio.example.com/bucket/notebooks/main",
        vec![RemoteProjectionFile::new("notes/a.md", b"a").expect("file")],
    )
    .expect("request");

    let err = provider
        .request_binding(TransportCapability::Push, request.locator())
        .err()
        .expect("custom endpoint must fail before credentials resolve");

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
fn s3_custom_https_endpoint_source_acquisition_fails_before_credentials_resolve() {
    let transport = RecordingS3Transport::new(StatusCode::OK);
    let provider = S3ProjectionProvider {
        transport,
        credentials: S3CredentialSource::Fail("custom-endpoint-pull"),
        region: S3RegionSource::Fail("custom-endpoint-pull"),
        custom_profile: None,
        now,
    };
    let request = SourceAcquisitionRequest::new(
        RemoteProjectionProvider::S3,
        "s3+https://minio.example.com/bucket/notebooks/main",
    )
    .expect("request");

    let mut sink = TestCollectingSourceSink::default();
    let err = provider
        .acquire(request, &mut sink)
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

#[test]
fn s3_custom_https_endpoint_push_uses_explicit_profile_binding() {
    let _env = EnvGuard::set(&[
        ("MINIO_ACCESS_KEY_ID", Some("minio-key")),
        ("MINIO_SECRET_ACCESS_KEY", Some("minio-secret")),
        ("MINIO_SESSION_TOKEN", None),
    ]);
    let profile = RemoteProjectionS3Profile::env_profile(
        "minio",
        "https://minio.example.com",
        "bucket",
        "notebooks/main",
        "us-east-1",
        "MINIO",
        vec!["push".into(), "source-acquisition".into()],
    );
    let transport = RecordingS3Transport::new(StatusCode::OK);
    let provider = S3ProjectionProvider::new_for_test_with_profile(transport, profile, now);
    let request = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::S3,
        "s3+https://minio.example.com/bucket/notebooks/main",
        vec![RemoteProjectionFile::new("notes/a.md", b"a").expect("file")],
    )
    .expect("request");

    let binding = provider
        .request_binding(TransportCapability::Push, request.locator())
        .expect("binding");
    let outcome = push_request(
        &provider.transport,
        &binding.credentials,
        &binding.region,
        binding.custom_url_binding.as_ref(),
        provider.now,
        request,
    )
    .expect("custom endpoint push");

    assert_eq!(outcome.uploaded_files, 1);
    let calls = provider.transport.put_calls.lock().expect("put calls");
    assert_eq!(
        calls[0].url.as_str(),
        "https://minio.example.com/bucket/notebooks/main/notes/a.md"
    );
    assert!(header(&calls[0], "authorization").contains("Credential=minio-key/"));
    assert!(header(&calls[0], "authorization").contains("/us-east-1/s3/aws4_request"));
}

#[test]
fn s3_custom_https_endpoint_profile_env_ref_is_not_default_aws_fallback() {
    let _env = EnvGuard::set(&[
        ("AWS_ACCESS_KEY_ID", Some("aws-key")),
        ("AWS_SECRET_ACCESS_KEY", Some("aws-secret")),
        ("MINIO_ACCESS_KEY_ID", None),
        ("MINIO_SECRET_ACCESS_KEY", None),
        ("MINIO_SESSION_TOKEN", None),
    ]);
    let profile = RemoteProjectionS3Profile::env_profile(
        "minio",
        "https://minio.example.com",
        "bucket",
        "notebooks/main",
        "us-east-1",
        "MINIO",
        vec!["push".into()],
    );
    let transport = RecordingS3Transport::new(StatusCode::OK);
    let provider = S3ProjectionProvider::new_for_test_with_profile(transport, profile, now);
    let request = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::S3,
        "s3+https://minio.example.com/bucket/notebooks/main",
        vec![RemoteProjectionFile::new("notes/a.md", b"a").expect("file")],
    )
    .expect("request");

    let err = provider
        .request_binding(TransportCapability::Push, request.locator())
        .err()
        .expect("profile env ref must be explicit");

    assert!(err.to_string().contains("MINIO_ACCESS_KEY_ID"));
    assert!(!err.to_string().contains("AWS_ACCESS_KEY_ID"));
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

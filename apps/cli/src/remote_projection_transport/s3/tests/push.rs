use super::super::provider::S3ProjectionProvider;
use super::super::push::{S3ProjectionPushAdapter, push_request};
use super::support::{RecordingS3Transport, header, now, test_credentials};
use crate::remote_projection_transport::WorkspaceProjectionPushSource;
use deve_core::remote_projection::{
    RemoteProjectionFile, RemoteProjectionProvider, RemoteProjectionPushRequest,
};
use reqwest::StatusCode;
use std::fs;

#[test]
fn s3_push_puts_projection_files_without_authority_effects() {
    let transport = RecordingS3Transport::new(StatusCode::OK);
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let request = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::S3,
        "s3://bucket/notebooks/main",
        vec![
            RemoteProjectionFile::new("notes/a.md", b"a").expect("a"),
            RemoteProjectionFile::new("root.md", b"root").expect("root"),
        ],
    )
    .expect("request");

    let binding = provider
        .request_binding(
            crate::remote_projection_transport::TransportCapability::Push,
            request.locator(),
        )
        .expect("binding");
    let outcome = push_request(
        &provider.transport,
        &binding.credentials,
        &binding.region,
        binding.custom_url_binding.as_ref(),
        provider.now,
        request,
    )
    .expect("push");

    assert_eq!(outcome.uploaded_files, 2);
    assert!(!outcome.effects.writes_ledger);
    assert!(!outcome.effects.writes_source_control_staging);
    assert!(!outcome.effects.writes_commit_anchor);
    assert!(!outcome.effects.writes_git_main_mirror);
    assert!(!outcome.effects.confirms_external_changes);
    assert!(outcome.provider_metadata_is_diagnostic_only);
    let calls = provider.transport.put_calls.lock().expect("calls");
    assert_eq!(
        calls
            .iter()
            .map(|call| call.url.as_str())
            .collect::<Vec<_>>(),
        vec![
            "https://bucket.s3.us-east-1.amazonaws.com/notebooks/main/notes/a.md",
            "https://bucket.s3.us-east-1.amazonaws.com/notebooks/main/root.md",
        ]
    );
    assert_eq!(calls[0].body, b"a");
    assert_eq!(calls[1].body, b"root");
    assert!(header(&calls[0], "authorization").contains("AWS4-HMAC-SHA256"));
    assert!(header(&calls[0], "authorization").contains("/us-east-1/s3/aws4_request"));
    assert_eq!(header(&calls[0], "x-amz-content-sha256").len(), 64);
}

#[test]
fn s3_push_rejects_failed_put() {
    let transport = RecordingS3Transport::new(StatusCode::INTERNAL_SERVER_ERROR);
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let request = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::S3,
        "s3://bucket/notebooks/main",
        vec![RemoteProjectionFile::new("a.md", b"a").expect("a")],
    )
    .expect("request");

    let binding = provider
        .request_binding(
            crate::remote_projection_transport::TransportCapability::Push,
            request.locator(),
        )
        .expect("binding");
    let err = push_request(
        &provider.transport,
        &binding.credentials,
        &binding.region,
        binding.custom_url_binding.as_ref(),
        provider.now,
        request,
    )
    .expect_err("put failure");

    assert!(err.to_string().contains("S3 PUT a.md failed"));
}

#[test]
fn s3_streaming_put_failure_is_not_provider_unavailable() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("a.md"), "a").expect("a");
    let source = WorkspaceProjectionPushSource::collect(dir.path()).expect("source");
    let transport = RecordingS3Transport::new(StatusCode::INTERNAL_SERVER_ERROR);
    let mut provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);

    let error = provider
        .push_projection_files(
            RemoteProjectionProvider::S3,
            "s3://bucket/notebooks/main",
            &source,
        )
        .expect_err("put failure");

    assert!(!error.is_provider_unavailable());
    assert!(error.to_string().contains("S3 PUT a.md failed"));
}

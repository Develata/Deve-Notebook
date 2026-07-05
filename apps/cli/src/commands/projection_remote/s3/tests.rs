use super::credentials::S3Credentials;
use super::provider::S3ProjectionProvider;
use super::signing::S3SignedPutRequest;
use super::transport::S3Transport;
use super::url::s3_file_url;
use chrono::{TimeZone, Utc};
use deve_core::remote_projection::{
    RemoteProjectionFile, RemoteProjectionProvider, RemoteProjectionProviderAdapter,
    RemoteProjectionProviderError, RemoteProjectionPushRequest,
};
use reqwest::StatusCode;
use std::sync::Mutex;

#[test]
fn s3_push_puts_projection_files_without_authority_effects() {
    let transport = RecordingS3Transport::new(StatusCode::OK);
    let mut provider =
        S3ProjectionProvider::new_for_test(transport, S3Credentials::for_test(), "us-east-1", now);
    let request = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::S3,
        "s3://bucket/notebooks/main",
        vec![
            RemoteProjectionFile::new("notes/a.md", b"a").expect("a"),
            RemoteProjectionFile::new("root.md", b"root").expect("root"),
        ],
    )
    .expect("request");

    let outcome = provider.push(request).expect("push");

    assert_eq!(outcome.uploaded_files, 2);
    assert!(!outcome.effects.writes_ledger);
    assert!(!outcome.effects.writes_source_control_staging);
    assert!(!outcome.effects.writes_commit_anchor);
    assert!(!outcome.effects.writes_git_main_mirror);
    assert!(!outcome.effects.confirms_external_changes);
    assert!(outcome.provider_metadata_is_diagnostic_only);
    let calls = provider.transport.calls.lock().expect("calls");
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
    let mut provider =
        S3ProjectionProvider::new_for_test(transport, S3Credentials::for_test(), "us-east-1", now);
    let request = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::S3,
        "s3://bucket/notebooks/main",
        vec![RemoteProjectionFile::new("a.md", b"a").expect("a")],
    )
    .expect("request");

    let err = provider.push(request).expect_err("put failure");

    assert!(err.to_string().contains("S3 PUT a.md failed"));
}

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
}

#[test]
fn s3_signed_request_matches_golden_vector() {
    let url = s3_file_url("s3://bucket/notebooks/main", "us-east-1", "a.md").expect("url");
    let request = super::signing::signed_put_request(
        url,
        b"a".to_vec(),
        &S3Credentials::for_test(),
        "us-east-1",
        now(),
    )
    .expect("request");

    assert_eq!(
        header(&request, "x-amz-content-sha256"),
        "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
    );
    assert_eq!(header(&request, "x-amz-date"), "20260705T120000Z");
    assert_eq!(
        header(&request, "authorization"),
        "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20260705/us-east-1/s3/aws4_request, SignedHeaders=content-type;host;x-amz-content-sha256;x-amz-date, Signature=7128a98b8dfe318572aca62bb4f368deb9ca2044a92a7d4a8349e1565f190ffb"
    );
}

#[test]
fn s3_signed_request_changes_with_payload() {
    let url = s3_file_url("s3://bucket/notebooks/main", "us-east-1", "a.md").expect("url");
    let left = super::signing::signed_put_request(
        url.clone(),
        b"a".to_vec(),
        &S3Credentials::for_test(),
        "us-east-1",
        now(),
    )
    .expect("left");
    let right = super::signing::signed_put_request(
        url,
        b"b".to_vec(),
        &S3Credentials::for_test(),
        "us-east-1",
        now(),
    )
    .expect("right");

    assert_ne!(
        header(&left, "x-amz-content-sha256"),
        header(&right, "x-amz-content-sha256")
    );
    assert_ne!(
        header(&left, "authorization"),
        header(&right, "authorization")
    );
}

#[derive(Debug)]
struct RecordingS3Transport {
    calls: Mutex<Vec<S3SignedPutRequest>>,
    status: StatusCode,
}

impl RecordingS3Transport {
    fn new(status: StatusCode) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            status,
        }
    }
}

impl S3Transport for RecordingS3Transport {
    fn put(
        &self,
        request: S3SignedPutRequest,
    ) -> Result<StatusCode, RemoteProjectionProviderError> {
        self.calls.lock().expect("calls").push(request);
        Ok(self.status)
    }
}

fn header(request: &S3SignedPutRequest, name: &str) -> String {
    request
        .headers
        .iter()
        .find_map(|(header_name, value)| (header_name == name).then(|| value.clone()))
        .unwrap_or_else(|| panic!("missing header {name}"))
}

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 5, 12, 0, 0)
        .single()
        .expect("time")
}

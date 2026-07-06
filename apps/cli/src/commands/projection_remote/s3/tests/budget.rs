use super::super::provider::S3ProjectionProvider;
use super::super::pull::S3ProjectionPullAdapter;
use super::support::{RecordingS3Transport, now, s3_list_body, test_credentials};
use deve_core::remote_projection::RemoteProjectionProvider;
use reqwest::StatusCode;

#[test]
fn s3_pull_rejects_oversized_file_before_workspace_write() {
    let dir = tempfile::tempdir().expect("tempdir");
    let transport = RecordingS3Transport::new(StatusCode::OK)
        .with_get_body(s3_list_body(&["notebooks/main/big.md"], None))
        .with_get_body(vec![b'x'; super::super::pull::MAX_PULL_FILE_BYTES + 1]);
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);

    let err = provider
        .pull_projection_files(RemoteProjectionProvider::S3, "s3://bucket/notebooks/main")
        .expect_err("oversized body");

    assert!(err.to_string().contains("S3 response body exceeds"));
    assert!(!dir.path().join("big.md").exists());
}

#[test]
fn s3_pull_rejects_too_many_files_before_workspace_write() {
    let keys = (0..=super::super::list::MAX_PULL_FILES)
        .map(|index| format!("notebooks/main/{index:04}.md"))
        .collect::<Vec<_>>();
    let key_refs = keys.iter().map(String::as_str).collect::<Vec<_>>();
    let transport =
        RecordingS3Transport::new(StatusCode::OK).with_get_body(s3_list_body(&key_refs, None));
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);

    let err = provider
        .pull_projection_files(RemoteProjectionProvider::S3, "s3://bucket/notebooks/main")
        .expect_err("file budget");

    assert!(err.to_string().contains("S3 pull exceeds file budget"));
    // Only the ListObjectsV2 request should run; object GETs and workspace apply
    // both require a successful pull outcome.
    assert_eq!(provider.transport.get_calls.lock().expect("calls").len(), 1);
}

#[test]
fn s3_pull_rejects_total_download_budget_before_workspace_write() {
    let file_count =
        (super::super::pull::MAX_PULL_TOTAL_BYTES / super::super::pull::MAX_PULL_FILE_BYTES) + 1;
    let keys = (0..file_count)
        .map(|index| format!("notebooks/main/{index:04}.md"))
        .collect::<Vec<_>>();
    let key_refs = keys.iter().map(String::as_str).collect::<Vec<_>>();
    let mut transport =
        RecordingS3Transport::new(StatusCode::OK).with_get_body(s3_list_body(&key_refs, None));
    for _ in 0..file_count {
        transport = transport.with_get_body(vec![b'x'; super::super::pull::MAX_PULL_FILE_BYTES]);
    }
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);

    let err = provider
        .pull_projection_files(RemoteProjectionProvider::S3, "s3://bucket/notebooks/main")
        .expect_err("total byte budget");

    assert!(
        err.to_string()
            .contains("S3 pull exceeds total byte budget")
    );
    assert_eq!(
        provider.transport.get_calls.lock().expect("calls").len(),
        1 + file_count
    );
}

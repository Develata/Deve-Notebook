use super::super::provider::S3ProjectionProvider;
use super::support::{RecordingS3Transport, now, s3_list_body, test_credentials};
use crate::remote_projection_transport::{
    RemoteSourceAcquisition, SourceAcquisitionRequest, TestCollectingSourceSink,
};
use deve_core::remote_projection::RemoteProjectionProvider;
use reqwest::StatusCode;

#[test]
fn s3_source_acquisition_rejects_oversized_file() {
    let transport = RecordingS3Transport::new(StatusCode::OK)
        .with_get_body(s3_list_body(&["notebooks/main/big.md"], None))
        .with_get_body(vec![b'x'; super::super::source::MAX_SOURCE_FILE_BYTES + 1]);
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);

    let request =
        SourceAcquisitionRequest::new(RemoteProjectionProvider::S3, "s3://bucket/notebooks/main")
            .expect("request");
    let mut sink = TestCollectingSourceSink::default();
    let err = provider
        .acquire(request, &mut sink)
        .expect_err("oversized body");

    assert!(err.to_string().contains("S3 source payload exceeds"));
}

#[test]
fn s3_source_acquisition_rejects_too_many_files_before_payload_get() {
    let keys = (0..=crate::remote_projection_transport::MAX_SOURCE_FILES)
        .map(|index| format!("notebooks/main/{index:04}.md"))
        .collect::<Vec<_>>();
    let key_refs = keys.iter().map(String::as_str).collect::<Vec<_>>();
    let transport =
        RecordingS3Transport::new(StatusCode::OK).with_get_body(s3_list_body(&key_refs, None));
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);

    let request =
        SourceAcquisitionRequest::new(RemoteProjectionProvider::S3, "s3://bucket/notebooks/main")
            .expect("request");
    let mut sink = TestCollectingSourceSink::default();
    let err = provider
        .acquire(request, &mut sink)
        .expect_err("file budget");

    assert!(
        err.to_string()
            .contains("S3 source acquisition exceeds file budget")
    );
    // Only the ListObjectsV2 request should run; object GETs and workspace apply
    // both require a successful pull outcome.
    assert_eq!(provider.transport.get_calls.lock().expect("calls").len(), 1);
}

#[test]
fn s3_source_acquisition_rejects_oversized_path_before_payload_get() {
    let key = format!(
        "notebooks/main/{}.md",
        "a".repeat(crate::remote_projection_transport::MAX_SOURCE_PATH_BYTES)
    );
    let transport =
        RecordingS3Transport::new(StatusCode::OK).with_get_body(s3_list_body(&[&key], None));
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let request =
        SourceAcquisitionRequest::new(RemoteProjectionProvider::S3, "s3://bucket/notebooks/main")
            .expect("request");
    let mut sink = TestCollectingSourceSink::default();

    let error = provider
        .acquire(request, &mut sink)
        .expect_err("path budget");

    assert!(error.to_string().contains("path exceeds 1024 bytes"));
    assert!(sink.files.is_empty());
    assert_eq!(provider.transport.get_calls.lock().expect("calls").len(), 1);
}

#[test]
fn s3_source_acquisition_rejects_total_byte_budget() {
    let file_count = (super::super::source::MAX_SOURCE_TOTAL_BYTES
        / super::super::source::MAX_SOURCE_FILE_BYTES)
        + 1;
    let keys = (0..file_count)
        .map(|index| format!("notebooks/main/{index:04}.md"))
        .collect::<Vec<_>>();
    let key_refs = keys.iter().map(String::as_str).collect::<Vec<_>>();
    let mut transport =
        RecordingS3Transport::new(StatusCode::OK).with_get_body(s3_list_body(&key_refs, None));
    for _ in 0..file_count {
        transport =
            transport.with_get_body(vec![b'x'; super::super::source::MAX_SOURCE_FILE_BYTES]);
    }
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);

    let request =
        SourceAcquisitionRequest::new(RemoteProjectionProvider::S3, "s3://bucket/notebooks/main")
            .expect("request");
    let mut sink = TestCollectingSourceSink::default();
    let err = provider
        .acquire(request, &mut sink)
        .expect_err("total byte budget");

    assert!(
        err.to_string()
            .contains("S3 source acquisition exceeds total byte budget")
    );
    assert_eq!(sink.files.len(), file_count - 1);
    assert_eq!(
        provider.transport.get_calls.lock().expect("calls").len(),
        1 + file_count
    );
}

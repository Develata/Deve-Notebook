use super::super::provider::S3ProjectionProvider;
use super::support::{
    RecordingS3Transport, now, s3_list_body, s3_list_body_with_state,
    s3_truncated_list_body_without_token, test_credentials,
};
use crate::remote_projection_transport::{
    NormalizedRemotePath, RemoteSourceAcquisition, RemoteSourceSink, SourceAcquisitionError,
    SourceAcquisitionRequest, TestCollectingSourceSink,
};
use deve_core::remote_projection::{RemoteProjectionProvider, RemoteProjectionProviderError};
use reqwest::StatusCode;
use std::io::Read;

fn request() -> SourceAcquisitionRequest {
    SourceAcquisitionRequest::new(RemoteProjectionProvider::S3, "s3://bucket/notebooks/main")
        .expect("request")
}

#[test]
fn s3_source_acquisition_delivers_normalized_paths_in_order() {
    let transport = RecordingS3Transport::new(StatusCode::OK)
        .with_get_body(s3_list_body(
            &["notebooks/main/root.md", "notebooks/main/notes/a.md"],
            None,
        ))
        .with_get_body(b"a".to_vec())
        .with_get_body(b"root".to_vec());
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let mut sink = TestCollectingSourceSink::default();

    let outcome = provider.acquire(request(), &mut sink).expect("acquire");

    assert_eq!(outcome.files, 2);
    assert_eq!(outcome.bytes, 5);
    assert_eq!(
        sink.files
            .iter()
            .map(|file| (file.path(), file.content()))
            .collect::<Vec<_>>(),
        vec![
            ("notes/a.md", b"a".as_slice()),
            ("root.md", b"root".as_slice())
        ]
    );
}

#[test]
fn s3_source_acquisition_rejects_foreign_or_reserved_keys_before_payload_get() {
    for path in ["skip.txt", "StAgInG/secret.md", "CON.md", "bad?.md"] {
        let key = format!("notebooks/main/{path}");
        let transport =
            RecordingS3Transport::new(StatusCode::OK).with_get_body(s3_list_body(&[&key], None));
        let provider =
            S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
        let mut sink = TestCollectingSourceSink::default();

        provider
            .acquire(request(), &mut sink)
            .expect_err("foreign remote object");

        assert!(sink.files.is_empty());
        assert_eq!(
            provider.transport.get_calls.lock().expect("calls").len(),
            1,
            "{path}"
        );
    }
}

#[test]
fn s3_source_acquisition_rejects_failed_get() {
    let transport = RecordingS3Transport::new(StatusCode::OK)
        .with_get_body(s3_list_body(&["notebooks/main/a.md"], None))
        .with_get_response(StatusCode::INTERNAL_SERVER_ERROR, b"fail".to_vec());
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let mut sink = TestCollectingSourceSink::default();

    let error = provider
        .acquire(request(), &mut sink)
        .expect_err("GET failure");

    assert!(error.to_string().contains("S3 GET a.md failed"));
    assert!(sink.files.is_empty());
}

#[test]
fn s3_source_acquisition_rejects_partial_payload_get() {
    let transport = RecordingS3Transport::new(StatusCode::OK)
        .with_get_body(s3_list_body(&["notebooks/main/a.md"], None))
        .with_get_response(StatusCode::PARTIAL_CONTENT, b"partial".to_vec());
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let mut sink = TestCollectingSourceSink::default();

    provider
        .acquire(request(), &mut sink)
        .expect_err("partial payload");

    assert!(sink.files.is_empty());
}

#[test]
fn s3_source_acquisition_rejects_duplicate_paths_before_payload_get() {
    let transport = RecordingS3Transport::new(StatusCode::OK).with_get_body(s3_list_body(
        &["notebooks/main/a.md", "notebooks/main/a.md"],
        None,
    ));
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let mut sink = TestCollectingSourceSink::default();

    let error = provider
        .acquire(request(), &mut sink)
        .expect_err("duplicate");

    assert!(matches!(
        error,
        SourceAcquisitionError::Transport(RemoteProjectionProviderError::DuplicateProjectionPath)
    ));
    assert_eq!(provider.transport.get_calls.lock().expect("calls").len(), 1);
}

#[test]
fn s3_source_acquisition_rejects_casefold_collision_before_payload_get() {
    let transport = RecordingS3Transport::new(StatusCode::OK).with_get_body(s3_list_body(
        &["notebooks/main/A.md", "notebooks/main/a.md"],
        None,
    ));
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let mut sink = TestCollectingSourceSink::default();

    let error = provider
        .acquire(request(), &mut sink)
        .expect_err("collision");

    assert!(matches!(
        error,
        SourceAcquisitionError::Transport(RemoteProjectionProviderError::DuplicateProjectionPath)
    ));
    assert_eq!(provider.transport.get_calls.lock().expect("calls").len(), 1);
}

#[test]
fn s3_source_acquisition_paginates_before_ordered_payload_get() {
    let transport = RecordingS3Transport::new(StatusCode::OK)
        .with_get_body(s3_list_body(
            &["notebooks/main/root.md"],
            Some("next/token"),
        ))
        .with_get_body(s3_list_body(&["notebooks/main/notes/a.md"], None))
        .with_get_body(b"a".to_vec())
        .with_get_body(b"root".to_vec());
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let mut sink = TestCollectingSourceSink::default();

    provider.acquire(request(), &mut sink).expect("acquire");

    assert_eq!(
        sink.files
            .iter()
            .map(|file| file.path())
            .collect::<Vec<_>>(),
        vec!["notes/a.md", "root.md"]
    );
    let calls = provider.transport.get_calls.lock().expect("calls");
    assert_eq!(
        calls[1].url.as_str(),
        "https://bucket.s3.us-east-1.amazonaws.com/?continuation-token=next%2Ftoken&list-type=2&prefix=notebooks%2Fmain%2F"
    );
}

#[test]
fn s3_source_acquisition_decodes_xml_entities() {
    let transport = RecordingS3Transport::new(StatusCode::OK)
        .with_get_body(s3_list_body(
            &["notebooks/main/notes/a&b.md"],
            Some("next&token"),
        ))
        .with_get_body(s3_list_body(&["notebooks/main/root.md"], None))
        .with_get_body(b"amp".to_vec())
        .with_get_body(b"root".to_vec());
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let mut sink = TestCollectingSourceSink::default();

    provider.acquire(request(), &mut sink).expect("acquire");

    assert_eq!(
        sink.files
            .iter()
            .map(|file| file.path())
            .collect::<Vec<_>>(),
        vec!["notes/a&b.md", "root.md"]
    );
}

#[test]
fn s3_source_acquisition_rejects_truncated_list_without_token() {
    let transport = RecordingS3Transport::new(StatusCode::OK).with_get_body(
        s3_truncated_list_body_without_token(&["notebooks/main/root.md"]),
    );
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let mut sink = TestCollectingSourceSink::default();

    let error = provider
        .acquire(request(), &mut sink)
        .expect_err("truncated");

    assert!(error.to_string().contains("truncated without"));
}

#[test]
fn s3_source_acquisition_rejects_inconsistent_or_repeated_continuation_token() {
    let cases = [
        vec![s3_list_body_with_state(&[], false, Some("unexpected"))],
        vec![
            s3_list_body_with_state(&[], true, Some("repeat")),
            s3_list_body_with_state(&[], true, Some("repeat")),
        ],
    ];
    for responses in cases {
        let mut transport = RecordingS3Transport::new(StatusCode::OK);
        for response in responses {
            transport = transport.with_get_body(response);
        }
        let provider =
            S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
        let mut sink = TestCollectingSourceSink::default();

        provider
            .acquire(request(), &mut sink)
            .expect_err("invalid continuation state");

        assert!(sink.files.is_empty());
        assert!(provider.transport.get_calls.lock().expect("calls").len() <= 2);
    }
}

#[test]
fn s3_source_acquisition_rejects_non_ok_or_incomplete_listing() {
    let cases = [
        (StatusCode::NO_CONTENT, Vec::new()),
        (StatusCode::PARTIAL_CONTENT, s3_list_body(&[], None)),
        (StatusCode::OK, Vec::new()),
        (StatusCode::OK, b"<html></html>".to_vec()),
        (
            StatusCode::OK,
            b"<ListBucketResult><IsTruncated>false</IsTruncated><Contents><Key>notebooks/main/a.md</Key></Contents>".to_vec(),
        ),
    ];
    for (status, response) in cases {
        let transport =
            RecordingS3Transport::new(StatusCode::OK).with_get_response(status, response);
        let provider =
            S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
        let mut sink = TestCollectingSourceSink::default();

        provider
            .acquire(request(), &mut sink)
            .expect_err("incomplete listing");

        assert!(sink.files.is_empty());
        assert_eq!(provider.transport.get_calls.lock().expect("calls").len(), 1);
    }
}

#[test]
fn s3_source_acquisition_rejects_object_outside_requested_prefix() {
    let transport = RecordingS3Transport::new(StatusCode::OK)
        .with_get_body(s3_list_body(&["other/a.md"], None));
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let mut sink = TestCollectingSourceSink::default();

    provider
        .acquire(request(), &mut sink)
        .expect_err("foreign prefix");

    assert!(sink.files.is_empty());
    assert_eq!(provider.transport.get_calls.lock().expect("calls").len(), 1);
}

#[derive(Default)]
struct FailingSink {
    calls: usize,
}

impl RemoteSourceSink for FailingSink {
    type Error = &'static str;

    fn capture(
        &mut self,
        _path: &NormalizedRemotePath,
        _body: &mut dyn Read,
    ) -> Result<(), Self::Error> {
        self.calls += 1;
        Err("capture rejected")
    }
}

#[test]
fn s3_source_acquisition_stops_after_sink_failure() {
    let transport = RecordingS3Transport::new(StatusCode::OK)
        .with_get_body(s3_list_body(
            &["notebooks/main/a.md", "notebooks/main/b.md"],
            None,
        ))
        .with_get_body(b"a".to_vec())
        .with_get_body(b"b".to_vec());
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let mut sink = FailingSink::default();

    let error = provider
        .acquire(request(), &mut sink)
        .expect_err("sink failure");

    assert!(matches!(
        error,
        SourceAcquisitionError::Sink("capture rejected")
    ));
    assert_eq!(sink.calls, 1);
    assert_eq!(provider.transport.get_calls.lock().expect("calls").len(), 2);
}

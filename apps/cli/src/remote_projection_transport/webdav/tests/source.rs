use super::super::WebDavProjectionProvider;
use super::support::{RecordingTransport, propfind_body};
use crate::remote_projection_transport::{
    NormalizedRemotePath, RemoteSourceAcquisition, RemoteSourceSink, SourceAcquisitionError,
    SourceAcquisitionRequest, TestCollectingSourceSink,
};
use deve_core::remote_projection::{RemoteProjectionProvider, RemoteProjectionProviderError};
use reqwest::StatusCode;
use std::io::Read;

#[test]
fn webdav_source_acquisition_delivers_normalized_paths_in_order() {
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&[
            ("https://dav.example.com/notebooks/main", true),
            ("https://dav.example.com/notebooks/main/root.md", false),
            ("https://dav.example.com/notebooks/main/notes", true),
        ]))
        .with_propfind_body(propfind_body(&[
            ("https://dav.example.com/notebooks/main/notes/", true),
            ("https://dav.example.com/notebooks/main/notes/a.md", false),
        ]))
        .with_get_body(b"a".to_vec())
        .with_get_body(b"root".to_vec());
    let provider = WebDavProjectionProvider::new_for_test(transport);
    let request = SourceAcquisitionRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
    )
    .expect("request");
    let mut sink = TestCollectingSourceSink::default();

    let outcome = provider.acquire(request, &mut sink).expect("acquire");

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
fn webdav_source_acquisition_rejects_foreign_or_reserved_files_before_payload_get() {
    for path in ["skip.txt", "SNAPSHOTS/secret.md", "CON.md", "bad%3F.md"] {
        let href = format!("https://dav.example.com/notebooks/main/{path}");
        let transport =
            RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
                .with_propfind_body(propfind_body(&[(&href, false)]));
        let provider = WebDavProjectionProvider::new_for_test(transport);
        let request = SourceAcquisitionRequest::new(
            RemoteProjectionProvider::WebDav,
            "webdav+https://dav.example.com/notebooks/main",
        )
        .expect("request");
        let mut sink = TestCollectingSourceSink::default();

        provider
            .acquire(request, &mut sink)
            .expect_err("foreign remote object");

        assert!(sink.files.is_empty());
        assert!(
            provider
                .transport
                .calls
                .lock()
                .expect("calls")
                .iter()
                .all(|call| !call.starts_with("GET ")),
            "{path}"
        );
    }
}

#[test]
fn webdav_source_acquisition_rejects_failed_get() {
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&[(
            "https://dav.example.com/notebooks/main/a.md",
            false,
        )]))
        .with_get_response(StatusCode::INTERNAL_SERVER_ERROR, b"fail".to_vec());
    let provider = WebDavProjectionProvider::new_for_test(transport);
    let request = SourceAcquisitionRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
    )
    .expect("request");
    let mut sink = TestCollectingSourceSink::default();

    let error = provider
        .acquire(request, &mut sink)
        .expect_err("GET failure");

    assert!(error.to_string().contains("WebDAV GET a.md failed"));
    assert!(sink.files.is_empty());
}

#[test]
fn webdav_source_acquisition_rejects_partial_payload_get() {
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&[(
            "https://dav.example.com/notebooks/main/a.md",
            false,
        )]))
        .with_get_response(StatusCode::PARTIAL_CONTENT, b"partial".to_vec());
    let provider = WebDavProjectionProvider::new_for_test(transport);
    let request = SourceAcquisitionRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
    )
    .expect("request");
    let mut sink = TestCollectingSourceSink::default();

    provider
        .acquire(request, &mut sink)
        .expect_err("partial payload");

    assert!(sink.files.is_empty());
}

#[test]
fn webdav_source_acquisition_decodes_xml_entities() {
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&[(
            "https://dav.example.com/notebooks/main/a&amp;b.md",
            false,
        )]))
        .with_get_body(b"amp".to_vec());
    let provider = WebDavProjectionProvider::new_for_test(transport);
    let request = SourceAcquisitionRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
    )
    .expect("request");
    let mut sink = TestCollectingSourceSink::default();

    provider.acquire(request, &mut sink).expect("acquire");

    assert_eq!(sink.files.len(), 1);
    assert_eq!(sink.files[0].path(), "a&b.md");
    assert_eq!(sink.files[0].content(), b"amp");
}

#[test]
fn webdav_source_acquisition_rejects_duplicate_paths_before_payload_get() {
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&[
            ("https://dav.example.com/notebooks/main/a.md", false),
            ("https://dav.example.com/notebooks/main/a.md", false),
        ]));
    let provider = WebDavProjectionProvider::new_for_test(transport);
    let request = SourceAcquisitionRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
    )
    .expect("request");
    let mut sink = TestCollectingSourceSink::default();

    let error = provider.acquire(request, &mut sink).expect_err("duplicate");

    assert!(matches!(
        error,
        SourceAcquisitionError::Transport(RemoteProjectionProviderError::DuplicateProjectionPath)
    ));
    assert!(
        provider
            .transport
            .calls
            .lock()
            .expect("calls")
            .iter()
            .all(|call| !call.starts_with("GET "))
    );
}

#[test]
fn webdav_source_acquisition_rejects_casefold_collision_before_payload_get() {
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&[
            ("https://dav.example.com/notebooks/main/A.md", false),
            ("https://dav.example.com/notebooks/main/a.md", false),
        ]));
    let provider = WebDavProjectionProvider::new_for_test(transport);
    let request = SourceAcquisitionRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
    )
    .expect("request");
    let mut sink = TestCollectingSourceSink::default();

    let error = provider.acquire(request, &mut sink).expect_err("collision");

    assert!(matches!(
        error,
        SourceAcquisitionError::Transport(RemoteProjectionProviderError::DuplicateProjectionPath)
    ));
    assert!(
        provider
            .transport
            .calls
            .lock()
            .expect("calls")
            .iter()
            .all(|call| !call.starts_with("GET "))
    );
}

#[test]
fn webdav_source_acquisition_rejects_non_multistatus_or_incomplete_listing() {
    let cases = [
        (StatusCode::NO_CONTENT, Vec::new()),
        (StatusCode::PARTIAL_CONTENT, propfind_body(&[]).into_bytes()),
        (StatusCode::MULTI_STATUS, Vec::new()),
        (StatusCode::MULTI_STATUS, b"<html></html>".to_vec()),
        (
            StatusCode::MULTI_STATUS,
            br#"<d:multistatus xmlns:d="DAV:"><d:response><d:href>https://dav.example.com/notebooks/main/a.md</d:href></d:response>"#.to_vec(),
        ),
    ];
    for (status, response) in cases {
        let transport =
            RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
                .with_propfind_response(status, response);
        let provider = WebDavProjectionProvider::new_for_test(transport);
        let request = SourceAcquisitionRequest::new(
            RemoteProjectionProvider::WebDav,
            "webdav+https://dav.example.com/notebooks/main",
        )
        .expect("request");
        let mut sink = TestCollectingSourceSink::default();

        provider
            .acquire(request, &mut sink)
            .expect_err("incomplete listing");

        assert!(sink.files.is_empty());
        assert!(
            provider
                .transport
                .calls
                .lock()
                .expect("calls")
                .iter()
                .all(|call| !call.starts_with("GET "))
        );
    }
}

#[test]
fn webdav_source_acquisition_rejects_foreign_href_before_payload_get() {
    for href in [
        "https://evil.example.com/notebooks/main/a.md",
        "https://dav.example.com/notebooks/other/a.md",
    ] {
        let transport =
            RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
                .with_propfind_body(propfind_body(&[(href, false)]));
        let provider = WebDavProjectionProvider::new_for_test(transport);
        let request = SourceAcquisitionRequest::new(
            RemoteProjectionProvider::WebDav,
            "webdav+https://dav.example.com/notebooks/main",
        )
        .expect("request");
        let mut sink = TestCollectingSourceSink::default();

        provider
            .acquire(request, &mut sink)
            .expect_err("foreign href");

        assert!(sink.files.is_empty());
        assert!(
            provider
                .transport
                .calls
                .lock()
                .expect("calls")
                .iter()
                .all(|call| !call.starts_with("GET "))
        );
    }
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
fn webdav_source_acquisition_stops_after_sink_failure() {
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&[
            ("https://dav.example.com/notebooks/main/a.md", false),
            ("https://dav.example.com/notebooks/main/b.md", false),
        ]))
        .with_get_body(b"a".to_vec())
        .with_get_body(b"b".to_vec());
    let provider = WebDavProjectionProvider::new_for_test(transport);
    let request = SourceAcquisitionRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
    )
    .expect("request");
    let mut sink = FailingSink::default();

    let error = provider
        .acquire(request, &mut sink)
        .expect_err("sink failure");

    assert!(matches!(
        error,
        SourceAcquisitionError::Sink("capture rejected")
    ));
    assert_eq!(sink.calls, 1);
    assert_eq!(
        provider
            .transport
            .calls
            .lock()
            .expect("calls")
            .iter()
            .filter(|call| call.starts_with("GET "))
            .count(),
        1
    );
}

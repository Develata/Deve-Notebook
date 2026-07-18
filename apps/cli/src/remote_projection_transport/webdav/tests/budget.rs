use super::super::WebDavProjectionProvider;
use super::support::{RecordingTransport, propfind_body};
use crate::remote_projection_transport::{
    RemoteSourceAcquisition, SourceAcquisitionRequest, TestCollectingSourceSink,
};
use deve_core::remote_projection::RemoteProjectionProvider;
use reqwest::StatusCode;

#[test]
fn webdav_source_acquisition_rejects_oversized_file() {
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&[(
            "https://dav.example.com/notebooks/main/big.md",
            false,
        )]))
        .with_get_body(vec![b'x'; super::super::source::MAX_SOURCE_FILE_BYTES + 1]);
    let provider = WebDavProjectionProvider::new_for_test(transport);

    let request = SourceAcquisitionRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
    )
    .expect("request");
    let mut sink = TestCollectingSourceSink::default();
    let err = provider
        .acquire(request, &mut sink)
        .expect_err("oversized body");

    assert!(err.to_string().contains("WebDAV source payload exceeds"));
}

#[test]
fn webdav_source_acquisition_rejects_too_many_files_before_payload_get() {
    let hrefs = (0..=crate::remote_projection_transport::MAX_SOURCE_FILES)
        .map(|index| format!("https://dav.example.com/notebooks/main/{index:04}.md"))
        .collect::<Vec<_>>();
    let entries = hrefs
        .iter()
        .map(|href| (href.as_str(), false))
        .collect::<Vec<_>>();
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&entries));
    let provider = WebDavProjectionProvider::new_for_test(transport);

    let request = SourceAcquisitionRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
    )
    .expect("request");
    let mut sink = TestCollectingSourceSink::default();
    let err = provider
        .acquire(request, &mut sink)
        .expect_err("file budget");

    assert!(
        err.to_string()
            .contains("WebDAV source acquisition exceeds file budget")
    );
    // Only PROPFIND should run; file GETs and workspace apply both require a
    // successful pull outcome.
    assert_eq!(provider.transport.calls.lock().expect("calls").len(), 1);
}

#[test]
fn webdav_source_acquisition_rejects_collection_budget_before_enqueue_growth() {
    let hrefs = (0..super::super::source::MAX_SOURCE_COLLECTIONS)
        .map(|index| format!("https://dav.example.com/notebooks/main/{index:04}"))
        .collect::<Vec<_>>();
    let entries = hrefs
        .iter()
        .map(|href| (href.as_str(), true))
        .collect::<Vec<_>>();
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&entries));
    let provider = WebDavProjectionProvider::new_for_test(transport);
    let request = SourceAcquisitionRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
    )
    .expect("request");
    let mut sink = TestCollectingSourceSink::default();

    let error = provider
        .acquire(request, &mut sink)
        .expect_err("collection budget");

    assert!(error.to_string().contains("exceeds collection budget"));
    assert_eq!(provider.transport.calls.lock().expect("calls").len(), 1);
}

#[test]
fn webdav_source_acquisition_rejects_oversized_path_before_payload_get() {
    let path = format!(
        "https://dav.example.com/notebooks/main/{}.md",
        "a".repeat(crate::remote_projection_transport::MAX_SOURCE_PATH_BYTES)
    );
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&[(&path, false)]));
    let provider = WebDavProjectionProvider::new_for_test(transport);
    let request = SourceAcquisitionRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
    )
    .expect("request");
    let mut sink = TestCollectingSourceSink::default();

    let error = provider
        .acquire(request, &mut sink)
        .expect_err("path budget");

    assert!(error.to_string().contains("path exceeds 1024 bytes"));
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

#[test]
fn webdav_source_acquisition_rejects_total_byte_budget() {
    let file_count = (super::super::source::MAX_SOURCE_TOTAL_BYTES
        / super::super::source::MAX_SOURCE_FILE_BYTES)
        + 1;
    let hrefs = (0..file_count)
        .map(|index| format!("https://dav.example.com/notebooks/main/{index:04}.md"))
        .collect::<Vec<_>>();
    let entries = hrefs
        .iter()
        .map(|href| (href.as_str(), false))
        .collect::<Vec<_>>();
    let mut transport =
        RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
            .with_propfind_body(propfind_body(&entries));
    for _ in 0..file_count {
        transport =
            transport.with_get_body(vec![b'x'; super::super::source::MAX_SOURCE_FILE_BYTES]);
    }
    let provider = WebDavProjectionProvider::new_for_test(transport);

    let request = SourceAcquisitionRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
    )
    .expect("request");
    let mut sink = TestCollectingSourceSink::default();
    let err = provider
        .acquire(request, &mut sink)
        .expect_err("total byte budget");

    assert!(
        err.to_string()
            .contains("WebDAV source acquisition exceeds total byte budget")
    );
    assert_eq!(sink.files.len(), file_count - 1);
    assert_eq!(
        provider.transport.calls.lock().expect("calls").len(),
        1 + file_count
    );
}

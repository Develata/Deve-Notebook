use super::super::WebDavProjectionProvider;
use super::super::push::{WebDavProjectionPushAdapter, push_request};
use super::support::RecordingTransport;
use crate::remote_projection_transport::WorkspaceProjectionPushSource;
use deve_core::remote_projection::{
    RemoteProjectionFile, RemoteProjectionProvider, RemoteProjectionPushRequest,
};
use reqwest::StatusCode;
use std::fs;

#[test]
fn webdav_push_puts_projection_files_without_authority_effects() {
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED);
    let provider = WebDavProjectionProvider::new_for_test(transport);
    let request = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
        vec![
            RemoteProjectionFile::new("notes/a.md", b"a").expect("a"),
            RemoteProjectionFile::new("root.md", b"root").expect("root"),
        ],
    )
    .expect("request");

    let outcome = push_request(&provider.transport, request).expect("push");

    assert_eq!(outcome.uploaded_files, 2);
    assert!(!outcome.effects.writes_ledger);
    assert!(!outcome.effects.writes_source_control_staging);
    assert!(!outcome.effects.writes_commit_anchor);
    assert!(!outcome.effects.writes_git_main_mirror);
    assert!(!outcome.effects.confirms_external_changes);
    assert!(outcome.provider_metadata_is_diagnostic_only);
    let calls = provider.transport.calls.lock().expect("calls").clone();
    assert_eq!(
        calls,
        vec![
            "MKCOL https://dav.example.com/notebooks/main".to_string(),
            "MKCOL https://dav.example.com/notebooks/main/notes".to_string(),
            "PUT https://dav.example.com/notebooks/main/notes/a.md a".to_string(),
            "PUT https://dav.example.com/notebooks/main/root.md root".to_string(),
        ]
    );
}

#[test]
fn webdav_push_rejects_failed_put() {
    let transport = RecordingTransport::new(
        StatusCode::METHOD_NOT_ALLOWED,
        StatusCode::INTERNAL_SERVER_ERROR,
    );
    let provider = WebDavProjectionProvider::new_for_test(transport);
    let request = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
        vec![RemoteProjectionFile::new("a.md", b"a").expect("a")],
    )
    .expect("request");

    let err = push_request(&provider.transport, request).expect_err("put failure");

    assert!(err.to_string().contains("WebDAV PUT a.md failed"));
}

#[test]
fn webdav_streaming_push_reads_files_one_at_a_time_without_authority_effects() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("a.md"), "a").expect("a");
    fs::write(dir.path().join("b.md"), "b").expect("b");
    let source = WorkspaceProjectionPushSource::collect(dir.path()).expect("source");
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED);
    let mut provider = WebDavProjectionProvider::new_for_test(transport);

    let outcome = provider
        .push_projection_files(
            RemoteProjectionProvider::WebDav,
            "webdav+https://dav.example.com/notebooks/main",
            &source,
        )
        .expect("push");

    assert_eq!(outcome.uploaded_files, 2);
    assert!(!outcome.effects.writes_ledger);
    assert!(!outcome.effects.writes_source_control_staging);
    assert!(!outcome.effects.writes_commit_anchor);
    assert!(!outcome.effects.writes_git_main_mirror);
    assert!(!outcome.effects.confirms_external_changes);
    let calls = provider.transport.calls.lock().expect("calls").clone();
    assert_eq!(
        calls,
        vec![
            "MKCOL https://dav.example.com/notebooks/main".to_string(),
            "PUT https://dav.example.com/notebooks/main/a.md a".to_string(),
            "PUT https://dav.example.com/notebooks/main/b.md b".to_string(),
        ]
    );
}

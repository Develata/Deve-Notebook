use super::super::{WebDavProjectionProvider, WebDavProjectionPullAdapter};
use super::support::{RecordingTransport, propfind_body};
use deve_core::remote_projection::RemoteProjectionProvider;
use reqwest::StatusCode;

#[test]
fn webdav_pull_rejects_oversized_file_before_workspace_write() {
    let dir = tempfile::tempdir().expect("tempdir");
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&[(
            "https://dav.example.com/notebooks/main/big.md",
            false,
        )]))
        .with_get_body(vec![b'x'; super::super::pull::MAX_PULL_FILE_BYTES + 1]);
    let provider = WebDavProjectionProvider::new_for_test(transport);

    let err = provider
        .pull_projection_files(
            RemoteProjectionProvider::WebDav,
            "webdav+https://dav.example.com/notebooks/main",
        )
        .expect_err("oversized body");

    assert!(err.to_string().contains("WebDAV response body exceeds"));
    assert!(!dir.path().join("big.md").exists());
}

#[test]
fn webdav_pull_rejects_too_many_files_before_workspace_write() {
    let hrefs = (0..=super::super::pull::MAX_PULL_FILES)
        .map(|index| format!("https://dav.example.com/notebooks/main/{index:04}.md"))
        .collect::<Vec<_>>();
    let entries = hrefs
        .iter()
        .map(|href| (href.as_str(), false))
        .collect::<Vec<_>>();
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&entries));
    let provider = WebDavProjectionProvider::new_for_test(transport);

    let err = provider
        .pull_projection_files(
            RemoteProjectionProvider::WebDav,
            "webdav+https://dav.example.com/notebooks/main",
        )
        .expect_err("file budget");

    assert!(err.to_string().contains("WebDAV pull exceeds file budget"));
    // Only PROPFIND should run; file GETs and workspace apply both require a
    // successful pull outcome.
    assert_eq!(provider.transport.calls.lock().expect("calls").len(), 1);
}

#[test]
fn webdav_pull_rejects_total_download_budget_before_workspace_write() {
    let file_count =
        (super::super::pull::MAX_PULL_TOTAL_BYTES / super::super::pull::MAX_PULL_FILE_BYTES) + 1;
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
        transport = transport.with_get_body(vec![b'x'; super::super::pull::MAX_PULL_FILE_BYTES]);
    }
    let provider = WebDavProjectionProvider::new_for_test(transport);

    let err = provider
        .pull_projection_files(
            RemoteProjectionProvider::WebDav,
            "webdav+https://dav.example.com/notebooks/main",
        )
        .expect_err("total byte budget");

    assert!(
        err.to_string()
            .contains("WebDAV pull exceeds total byte budget")
    );
    assert_eq!(
        provider.transport.calls.lock().expect("calls").len(),
        1 + file_count
    );
}

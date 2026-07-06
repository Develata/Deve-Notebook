use super::super::{WebDavProjectionProvider, WebDavProjectionPullAdapter};
use super::support::{RecordingTransport, make_dir_symlink, propfind_body};
use crate::commands::projection_remote::workspace_apply::write_pull_files;
use deve_core::remote_projection::{
    RemoteProjectionProvider, RemoteProjectionProviderAdapter, RemoteProjectionProviderError,
    RemoteProjectionPullRequest,
};
use reqwest::StatusCode;
use std::fs;

#[test]
fn webdav_pull_downloads_markdown_files_and_writes_projection_workspace_without_authority_effects()
{
    let dir = tempfile::tempdir().expect("tempdir");
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&[
            ("https://dav.example.com/notebooks/main", true),
            ("https://dav.example.com/notebooks/main/root.md", false),
            ("https://dav.example.com/notebooks/main/skip.txt", false),
            ("https://dav.example.com/notebooks/main/notes", true),
            (
                "https://dav.example.com/notebooks/main/.notegit/secret.md",
                false,
            ),
        ]))
        .with_propfind_body(propfind_body(&[(
            "https://dav.example.com/notebooks/main/notes/a.md",
            false,
        )]))
        .with_get_body(b"a".to_vec())
        .with_get_body(b"root".to_vec());
    let provider = WebDavProjectionProvider::new_for_test(transport);

    let outcome = provider
        .pull_projection_files(
            RemoteProjectionProvider::WebDav,
            "webdav+https://dav.example.com/notebooks/main",
        )
        .expect("pull");
    let applied = write_pull_files(dir.path(), &outcome.files).expect("workspace apply");
    applied.commit();

    assert_eq!(outcome.files.len(), 2);
    assert!(!outcome.effects.writes_ledger);
    assert!(!outcome.effects.writes_source_control_staging);
    assert!(!outcome.effects.writes_commit_anchor);
    assert!(!outcome.effects.writes_git_main_mirror);
    assert!(!outcome.effects.confirms_external_changes);
    assert!(outcome.overwrites_projection_workspace);
    assert!(outcome.external_changes_confirmation_required);
    assert_eq!(
        fs::read_to_string(dir.path().join("notes").join("a.md")).expect("a"),
        "a"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("root.md")).expect("root"),
        "root"
    );
    assert!(!dir.path().join(".notegit").join("secret.md").exists());
}

#[test]
fn webdav_pull_rejects_failed_get() {
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&[(
            "https://dav.example.com/notebooks/main/a.md",
            false,
        )]))
        .with_get_response(StatusCode::INTERNAL_SERVER_ERROR, b"fail".to_vec());
    let provider = WebDavProjectionProvider::new_for_test(transport);
    let request = RemoteProjectionPullRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
    )
    .expect("request");

    let err = provider.pull(request).expect_err("get failure");

    assert!(err.to_string().contains("WebDAV GET a.md failed"));
}

#[test]
fn webdav_pull_rejects_duplicate_remote_markdown_paths_before_payload_get() {
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&[
            ("https://dav.example.com/notebooks/main/a.md", false),
            ("https://dav.example.com/notebooks/main/a.md", false),
        ]));
    let provider = WebDavProjectionProvider::new_for_test(transport);
    let request = RemoteProjectionPullRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
    )
    .expect("request");

    let err = provider.pull(request).expect_err("duplicate remote path");

    assert_eq!(err, RemoteProjectionProviderError::DuplicateProjectionPath);
    assert!(
        provider
            .transport
            .get_responses
            .lock()
            .expect("get responses")
            .is_empty()
    );
}

#[test]
fn webdav_pull_rejects_partial_apply_without_overwriting_existing_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("a-good.md"), "old").expect("old");
    fs::write(dir.path().join("blocked"), "not a directory").expect("blocked");
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&[
            ("https://dav.example.com/notebooks/main/a-good.md", false),
            (
                "https://dav.example.com/notebooks/main/blocked/new.md",
                false,
            ),
        ]))
        .with_get_body(b"new".to_vec())
        .with_get_body(b"blocked".to_vec());
    let provider = WebDavProjectionProvider::new_for_test(transport);

    let outcome = provider
        .pull_projection_files(
            RemoteProjectionProvider::WebDav,
            "webdav+https://dav.example.com/notebooks/main",
        )
        .expect("pull files");
    let err = write_pull_files(dir.path(), &outcome.files).expect_err("blocked parent");

    assert!(
        err.to_string()
            .contains("projection parent is not a directory")
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("a-good.md")).expect("good"),
        "old"
    );
    assert!(!dir.path().join("blocked").join("new.md").exists());
}

#[test]
fn webdav_pull_rejects_symlinked_parent_when_supported() {
    let dir = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside");
    if make_dir_symlink(outside.path(), &dir.path().join("linked")).is_err() {
        return;
    }
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED)
        .with_propfind_body(propfind_body(&[(
            "https://dav.example.com/notebooks/main/linked/a.md",
            false,
        )]))
        .with_get_body(b"escape".to_vec());
    let provider = WebDavProjectionProvider::new_for_test(transport);

    let outcome = provider
        .pull_projection_files(
            RemoteProjectionProvider::WebDav,
            "webdav+https://dav.example.com/notebooks/main",
        )
        .expect("pull files");
    let err = write_pull_files(dir.path(), &outcome.files).expect_err("symlink parent");

    assert!(err.to_string().contains("symlink"));
    assert!(!outside.path().join("a.md").exists());
}

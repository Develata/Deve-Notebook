use super::*;
use crate::remote_projection::RemoteProjectionError;

mod support;

use support::FakeRemoteProjectionProvider;

fn file(path: &str, content: &str) -> RemoteProjectionFile {
    RemoteProjectionFile::new(path, content.as_bytes()).expect("projection file")
}

#[test]
fn remote_projection_file_accepts_only_markdown_projection_paths() {
    let file = RemoteProjectionFile::new("notes\\daily.md", b"daily").expect("markdown path");
    assert_eq!(file.path(), "notes/daily.md");
    assert_eq!(file.content(), b"daily");

    assert_eq!(
        RemoteProjectionFile::new("notes/raw.txt", b"raw").expect_err("non markdown"),
        RemoteProjectionProviderError::InvalidProjectionPath
    );
    assert_eq!(
        RemoteProjectionFile::new("../escape.md", b"escape").expect_err("relative escape"),
        RemoteProjectionProviderError::InvalidProjectionPath
    );
    assert_eq!(
        RemoteProjectionFile::new(".git/config.md", b"git").expect_err("git internal"),
        RemoteProjectionProviderError::InternalStatePath
    );
    assert_eq!(
        RemoteProjectionFile::new("notes/.git/config.md", b"nested git")
            .expect_err("nested git internal"),
        RemoteProjectionProviderError::InternalStatePath
    );
    assert_eq!(
        RemoteProjectionFile::new(".notegit/state.md", b"notegit").expect_err("notegit internal"),
        RemoteProjectionProviderError::InternalStatePath
    );
    assert_eq!(
        RemoteProjectionFile::new("ledger/local.md", b"ledger").expect_err("ledger internal"),
        RemoteProjectionProviderError::InternalStatePath
    );
}

#[test]
fn provider_request_reuses_transport_admission_validator() {
    let wrong_scheme = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::WebDav,
        "s3://bucket/notebooks/main",
        vec![file("notes/a.md", "a")],
    )
    .expect_err("wrong scheme");
    assert_eq!(
        wrong_scheme,
        RemoteProjectionProviderError::AdmissionRejected(
            RemoteProjectionError::ProviderSchemeMismatch
        )
    );

    let secret_locator = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::S3,
        "s3://token@bucket/notebooks/main",
        Vec::new(),
    )
    .expect_err("secret locator");
    assert_eq!(
        secret_locator,
        RemoteProjectionProviderError::AdmissionRejected(
            RemoteProjectionError::SecretMaterialForbidden
        )
    );
}

#[test]
fn fake_adapter_push_stores_projection_files_without_authority_effects() {
    let mut adapter = FakeRemoteProjectionProvider::new(RemoteProjectionProvider::WebDav);

    let outcome = adapter
        .push(
            RemoteProjectionPushRequest::new(
                RemoteProjectionProvider::WebDav,
                " webdav+https://dav.example.com/notebooks/main ",
                vec![file("notes/a.md", "a"), file("notes/b.markdown", "b")],
            )
            .expect("request"),
        )
        .expect("push");

    assert_eq!(outcome.uploaded_files, 2);
    assert!(!outcome.effects.writes_ledger);
    assert!(!outcome.effects.writes_source_control_staging);
    assert!(!outcome.effects.writes_commit_anchor);
    assert!(!outcome.effects.writes_git_main_mirror);
    assert!(!outcome.effects.confirms_external_changes);
    assert!(outcome.provider_metadata_is_diagnostic_only);
    assert_eq!(
        adapter
            .remote_files("webdav+https://dav.example.com/notebooks/main")
            .expect("remote files")
            .len(),
        2
    );
}

#[test]
fn fake_adapter_push_rejects_provider_mismatch() {
    let mut adapter = FakeRemoteProjectionProvider::new(RemoteProjectionProvider::S3);
    let request = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
        vec![file("notes/a.md", "a")],
    )
    .expect("request");

    assert_eq!(
        adapter.push(request).expect_err("provider mismatch"),
        RemoteProjectionProviderError::ProviderMismatch
    );
}

#[test]
fn provider_request_rejects_duplicate_paths() {
    let duplicate = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
        vec![file("notes/a.md", "a"), file("notes/a.md", "again")],
    )
    .expect_err("duplicate path");
    assert_eq!(
        duplicate,
        RemoteProjectionProviderError::DuplicateProjectionPath
    );
}

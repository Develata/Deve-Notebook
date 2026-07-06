use super::super::provider::S3ProjectionProvider;
use super::super::pull::S3ProjectionPullAdapter;
use super::support::{
    RecordingS3Transport, now, s3_list_body, s3_truncated_list_body_without_token, test_credentials,
};
use crate::commands::projection_remote::workspace_apply::write_pull_files;
use deve_core::remote_projection::{
    RemoteProjectionProvider, RemoteProjectionProviderAdapter, RemoteProjectionProviderError,
    RemoteProjectionPullRequest,
};
use reqwest::StatusCode;
use std::fs;

#[test]
fn s3_pull_downloads_markdown_files_and_writes_projection_workspace_without_authority_effects() {
    let dir = tempfile::tempdir().expect("tempdir");
    let transport = RecordingS3Transport::new(StatusCode::OK)
        .with_get_body(s3_list_body(
            &[
                "notebooks/main/root.md",
                "notebooks/main/skip.txt",
                "notebooks/main/notes/a.md",
                "notebooks/main/.notegit/secret.md",
            ],
            None,
        ))
        .with_get_body(b"a".to_vec())
        .with_get_body(b"root".to_vec());
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);

    let outcome = provider
        .pull_projection_files(RemoteProjectionProvider::S3, "s3://bucket/notebooks/main")
        .expect("pull");
    let applied = write_pull_files(dir.path(), &outcome.files).expect("workspace apply");
    applied.commit();

    assert_eq!(outcome.files.len(), 2);
    assert!(!outcome.effects.writes_ledger);
    assert!(!outcome.effects.writes_source_control_staging);
    assert!(!outcome.effects.writes_commit_anchor);
    assert!(!outcome.effects.writes_git_main_mirror);
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
    let calls = provider.transport.get_calls.lock().expect("get calls");
    assert_eq!(
        calls
            .iter()
            .map(|call| call.url.as_str())
            .collect::<Vec<_>>(),
        vec![
            "https://bucket.s3.us-east-1.amazonaws.com/?list-type=2&prefix=notebooks%2Fmain%2F",
            "https://bucket.s3.us-east-1.amazonaws.com/notebooks/main/notes/a.md",
            "https://bucket.s3.us-east-1.amazonaws.com/notebooks/main/root.md",
        ]
    );
}

#[test]
fn s3_pull_rejects_failed_get() {
    let transport = RecordingS3Transport::new(StatusCode::OK)
        .with_get_body(s3_list_body(&["notebooks/main/a.md"], None))
        .with_get_response(StatusCode::INTERNAL_SERVER_ERROR, b"fail".to_vec());
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let request = RemoteProjectionPullRequest::new(
        RemoteProjectionProvider::S3,
        "s3://bucket/notebooks/main",
    )
    .expect("request");

    let err = provider.pull(request).expect_err("get failure");

    assert!(err.to_string().contains("S3 GET a.md failed"));
}

#[test]
fn s3_pull_rejects_duplicate_remote_markdown_paths_before_payload_get() {
    let transport = RecordingS3Transport::new(StatusCode::OK).with_get_body(s3_list_body(
        &["notebooks/main/a.md", "notebooks/main/a.md"],
        None,
    ));
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let request = RemoteProjectionPullRequest::new(
        RemoteProjectionProvider::S3,
        "s3://bucket/notebooks/main",
    )
    .expect("request");

    let err = provider.pull(request).expect_err("duplicate remote path");

    assert_eq!(err, RemoteProjectionProviderError::DuplicateProjectionPath);
    assert_eq!(
        provider
            .transport
            .get_calls
            .lock()
            .expect("get calls")
            .len(),
        1
    );
}

#[test]
fn s3_pull_rejects_partial_apply_without_overwriting_existing_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("a-good.md"), "old").expect("old");
    fs::write(dir.path().join("blocked"), "not a directory").expect("blocked");
    let transport = RecordingS3Transport::new(StatusCode::OK)
        .with_get_body(s3_list_body(
            &["notebooks/main/a-good.md", "notebooks/main/blocked/new.md"],
            None,
        ))
        .with_get_body(b"new".to_vec())
        .with_get_body(b"blocked".to_vec());
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);

    let outcome = provider
        .pull_projection_files(RemoteProjectionProvider::S3, "s3://bucket/notebooks/main")
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
fn s3_pull_paginates_list_objects() {
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
    let request = RemoteProjectionPullRequest::new(
        RemoteProjectionProvider::S3,
        "s3://bucket/notebooks/main",
    )
    .expect("request");

    let outcome = provider.pull(request).expect("pull");

    assert_eq!(
        outcome
            .files
            .iter()
            .map(|file| file.path().to_string())
            .collect::<Vec<_>>(),
        vec!["notes/a.md", "root.md"]
    );
    let calls = provider.transport.get_calls.lock().expect("get calls");
    assert_eq!(
        calls[1].url.as_str(),
        "https://bucket.s3.us-east-1.amazonaws.com/?continuation-token=next%2Ftoken&list-type=2&prefix=notebooks%2Fmain%2F"
    );
}

#[test]
fn s3_pull_decodes_xml_entities_in_keys_and_continuation_tokens() {
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
    let request = RemoteProjectionPullRequest::new(
        RemoteProjectionProvider::S3,
        "s3://bucket/notebooks/main",
    )
    .expect("request");

    let outcome = provider.pull(request).expect("pull");

    assert_eq!(
        outcome
            .files
            .iter()
            .map(|file| file.path().to_string())
            .collect::<Vec<_>>(),
        vec!["notes/a&b.md", "root.md"]
    );
    let calls = provider.transport.get_calls.lock().expect("get calls");
    assert_eq!(
        calls[1].url.as_str(),
        "https://bucket.s3.us-east-1.amazonaws.com/?continuation-token=next%26token&list-type=2&prefix=notebooks%2Fmain%2F"
    );
    assert_eq!(
        calls[2].url.as_str(),
        "https://bucket.s3.us-east-1.amazonaws.com/notebooks/main/notes/a&b.md"
    );
}

#[test]
fn s3_pull_rejects_truncated_list_without_continuation_token() {
    let transport = RecordingS3Transport::new(StatusCode::OK).with_get_body(
        s3_truncated_list_body_without_token(&["notebooks/main/root.md"]),
    );
    let provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let request = RemoteProjectionPullRequest::new(
        RemoteProjectionProvider::S3,
        "s3://bucket/notebooks/main",
    )
    .expect("request");

    let err = provider.pull(request).expect_err("truncated without token");

    assert!(err.to_string().contains("truncated without"));
}

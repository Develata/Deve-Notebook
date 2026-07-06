use super::provider::S3ProjectionProvider;
use super::pull::S3ProjectionPullAdapter;
use super::url::{s3_file_url, s3_list_url};
use crate::commands::projection_remote::workspace_apply::write_pull_files;
use deve_core::remote_projection::{
    RemoteProjectionFile, RemoteProjectionProvider, RemoteProjectionProviderAdapter,
    RemoteProjectionProviderError, RemoteProjectionPullRequest, RemoteProjectionPushRequest,
};
use reqwest::StatusCode;
use std::fs;

mod support;
use support::{
    RecordingS3Transport, get_header, header, now, s3_list_body,
    s3_truncated_list_body_without_token, test_credentials,
};

mod budget;

#[test]
fn s3_push_puts_projection_files_without_authority_effects() {
    let transport = RecordingS3Transport::new(StatusCode::OK);
    let mut provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let request = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::S3,
        "s3://bucket/notebooks/main",
        vec![
            RemoteProjectionFile::new("notes/a.md", b"a").expect("a"),
            RemoteProjectionFile::new("root.md", b"root").expect("root"),
        ],
    )
    .expect("request");

    let outcome = provider.push(request).expect("push");

    assert_eq!(outcome.uploaded_files, 2);
    assert!(!outcome.effects.writes_ledger);
    assert!(!outcome.effects.writes_source_control_staging);
    assert!(!outcome.effects.writes_commit_anchor);
    assert!(!outcome.effects.writes_git_main_mirror);
    assert!(!outcome.effects.confirms_external_changes);
    assert!(outcome.provider_metadata_is_diagnostic_only);
    let calls = provider.transport.put_calls.lock().expect("calls");
    assert_eq!(
        calls
            .iter()
            .map(|call| call.url.as_str())
            .collect::<Vec<_>>(),
        vec![
            "https://bucket.s3.us-east-1.amazonaws.com/notebooks/main/notes/a.md",
            "https://bucket.s3.us-east-1.amazonaws.com/notebooks/main/root.md",
        ]
    );
    assert_eq!(calls[0].body, b"a");
    assert_eq!(calls[1].body, b"root");
    assert!(header(&calls[0], "authorization").contains("AWS4-HMAC-SHA256"));
    assert!(header(&calls[0], "authorization").contains("/us-east-1/s3/aws4_request"));
    assert_eq!(header(&calls[0], "x-amz-content-sha256").len(), 64);
}

#[test]
fn s3_push_rejects_failed_put() {
    let transport = RecordingS3Transport::new(StatusCode::INTERNAL_SERVER_ERROR);
    let mut provider =
        S3ProjectionProvider::new_for_test(transport, test_credentials(), "us-east-1", now);
    let request = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::S3,
        "s3://bucket/notebooks/main",
        vec![RemoteProjectionFile::new("a.md", b"a").expect("a")],
    )
    .expect("request");

    let err = provider.push(request).expect_err("put failure");

    assert!(err.to_string().contains("S3 PUT a.md failed"));
}

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
fn s3_pull_rejects_duplicate_remote_markdown_paths_before_get() {
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

#[test]
fn s3_custom_https_endpoint_requires_explicit_credential_binding() {
    let err = s3_file_url(
        "s3+https://minio.example.com/bucket/notebooks/main",
        "unused-region",
        "notes/a.md",
    )
    .expect_err("custom endpoint must fail closed");

    assert!(err.to_string().contains("explicit credential binding"));
    assert!(err.to_string().contains("provider_io_ready=false"));

    let err = s3_list_url(
        "s3+https://minio.example.com/bucket/notebooks/main",
        "unused-region",
        None,
    )
    .expect_err("custom endpoint list must fail closed");
    assert!(err.to_string().contains("explicit credential binding"));
    assert!(err.to_string().contains("provider_io_ready=false"));
}

#[test]
fn s3_signed_request_matches_golden_vector() {
    let url = s3_file_url("s3://bucket/notebooks/main", "us-east-1", "a.md").expect("url");
    let request = super::signing::signed_put_request(
        url,
        b"a".to_vec(),
        &test_credentials(),
        "us-east-1",
        now(),
    )
    .expect("request");

    assert_eq!(
        header(&request, "x-amz-content-sha256"),
        "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
    );
    assert_eq!(header(&request, "x-amz-date"), "20260705T120000Z");
    assert_eq!(
        header(&request, "authorization"),
        "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20260705/us-east-1/s3/aws4_request, SignedHeaders=content-type;host;x-amz-content-sha256;x-amz-date, Signature=7128a98b8dfe318572aca62bb4f368deb9ca2044a92a7d4a8349e1565f190ffb"
    );
}

#[test]
fn s3_signed_get_request_includes_canonical_query() {
    let url = s3_list_url(
        "s3://bucket/notebooks/main",
        "us-east-1",
        Some("next/token"),
    )
    .expect("url");
    let request =
        super::signing::signed_get_request(url, &test_credentials(), "us-east-1", now(), 4096)
            .expect("request");

    assert_eq!(
        request.url.as_str(),
        "https://bucket.s3.us-east-1.amazonaws.com/?continuation-token=next%2Ftoken&list-type=2&prefix=notebooks%2Fmain%2F"
    );
    assert_eq!(
        get_header(&request, "authorization"),
        "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20260705/us-east-1/s3/aws4_request, SignedHeaders=host;x-amz-content-sha256;x-amz-date, Signature=d91974eb4a716deb25cef30fcc8166efa555bde3d48f6a3d9c1b02c1d10f4e26"
    );
}

#[test]
fn s3_signed_request_changes_with_payload() {
    let url = s3_file_url("s3://bucket/notebooks/main", "us-east-1", "a.md").expect("url");
    let left = super::signing::signed_put_request(
        url.clone(),
        b"a".to_vec(),
        &test_credentials(),
        "us-east-1",
        now(),
    )
    .expect("left");
    let right = super::signing::signed_put_request(
        url,
        b"b".to_vec(),
        &test_credentials(),
        "us-east-1",
        now(),
    )
    .expect("right");

    assert_ne!(
        header(&left, "x-amz-content-sha256"),
        header(&right, "x-amz-content-sha256")
    );
    assert_ne!(
        header(&left, "authorization"),
        header(&right, "authorization")
    );
}

use super::transport::{WebDavHttpResponse, WebDavTransport};
use super::*;
use crate::commands::projection_remote::workspace_apply::write_pull_files;
use deve_core::remote_projection::{
    RemoteProjectionFile, RemoteProjectionProvider, RemoteProjectionProviderAdapter,
    RemoteProjectionProviderError, RemoteProjectionPullRequest, RemoteProjectionPushRequest,
};
use reqwest::{StatusCode, Url};
use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

mod budget;

#[test]
fn collect_markdown_projection_files_uploads_only_markdown_projection_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(dir.path().join("notes")).expect("notes");
    fs::create_dir_all(dir.path().join(".notegit")).expect("notegit");
    fs::create_dir_all(dir.path().join(".git")).expect("git");
    fs::create_dir_all(dir.path().join("ledger")).expect("ledger");
    fs::write(dir.path().join("notes").join("a.md"), "a").expect("a");
    fs::write(dir.path().join("notes").join("b.markdown"), "b").expect("b");
    fs::write(dir.path().join("notes").join("skip.txt"), "skip").expect("txt");
    fs::write(dir.path().join(".notegit").join("secret.md"), "secret").expect("secret");
    fs::write(dir.path().join(".git").join("config.md"), "git").expect("git file");
    fs::write(dir.path().join("ledger").join("local.md"), "ledger").expect("ledger file");
    fs::write(dir.path().join(".deveignore"), "ignored.md\n").expect("ignore");
    fs::write(dir.path().join("ignored.md"), "ignored").expect("ignored");

    let files = collect_markdown_projection_files(dir.path()).expect("files");
    let paths = files
        .iter()
        .map(|file| file.path().to_string())
        .collect::<Vec<_>>();

    assert_eq!(paths, vec!["notes/a.md", "notes/b.markdown"]);
}

#[test]
fn collect_markdown_projection_files_skips_ignored_directories() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(dir.path().join("ignored_dir")).expect("ignored dir");
    fs::create_dir_all(dir.path().join("kept_dir")).expect("kept dir");
    fs::write(dir.path().join(".deveignore"), "ignored_dir/\n").expect("ignore");
    fs::write(dir.path().join("ignored_dir").join("secret.md"), "secret").expect("secret");
    fs::write(dir.path().join("kept_dir").join("note.md"), "note").expect("note");

    let files = collect_markdown_projection_files(dir.path()).expect("files");
    let paths = files
        .iter()
        .map(|file| file.path().to_string())
        .collect::<Vec<_>>();

    assert_eq!(paths, vec!["kept_dir/note.md"]);
}

#[test]
fn webdav_push_puts_projection_files_without_authority_effects() {
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED);
    let mut provider = WebDavProjectionProvider::new_for_test(transport);
    let request = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
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
    let mut provider = WebDavProjectionProvider::new_for_test(transport);
    let request = RemoteProjectionPushRequest::new(
        RemoteProjectionProvider::WebDav,
        "webdav+https://dav.example.com/notebooks/main",
        vec![RemoteProjectionFile::new("a.md", b"a").expect("a")],
    )
    .expect("request");

    let err = provider.push(request).expect_err("put failure");

    assert!(err.to_string().contains("WebDAV PUT a.md failed"));
}

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

#[test]
fn webdav_streaming_push_reads_files_one_at_a_time_without_authority_effects() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("a.md"), "a").expect("a");
    fs::write(dir.path().join("b.md"), "b").expect("b");
    let files = collect_markdown_projection_files(dir.path()).expect("files");
    let transport = RecordingTransport::new(StatusCode::METHOD_NOT_ALLOWED, StatusCode::CREATED);
    let mut provider = WebDavProjectionProvider::new_for_test(transport);

    let outcome = provider
        .push_projection_files(
            RemoteProjectionProvider::WebDav,
            "webdav+https://dav.example.com/notebooks/main",
            &files,
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

#[derive(Debug)]
struct RecordingTransport {
    calls: Mutex<Vec<String>>,
    mkcol_status: StatusCode,
    put_status: StatusCode,
    propfind_responses: Mutex<VecDeque<WebDavHttpResponse>>,
    get_responses: Mutex<VecDeque<WebDavHttpResponse>>,
}

impl RecordingTransport {
    fn new(mkcol_status: StatusCode, put_status: StatusCode) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            mkcol_status,
            put_status,
            propfind_responses: Mutex::new(VecDeque::new()),
            get_responses: Mutex::new(VecDeque::new()),
        }
    }

    fn with_propfind_body(self, body: String) -> Self {
        self.propfind_responses
            .lock()
            .expect("propfind")
            .push_back(WebDavHttpResponse {
                status: StatusCode::MULTI_STATUS,
                body: body.into_bytes(),
            });
        self
    }

    fn with_get_body(self, body: Vec<u8>) -> Self {
        self.with_get_response(StatusCode::OK, body)
    }

    fn with_get_response(self, status: StatusCode, body: Vec<u8>) -> Self {
        self.get_responses
            .lock()
            .expect("get")
            .push_back(WebDavHttpResponse { status, body });
        self
    }
}

impl WebDavTransport for RecordingTransport {
    fn mkcol(&self, url: &Url) -> Result<StatusCode, RemoteProjectionProviderError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("MKCOL {url}"));
        Ok(self.mkcol_status)
    }

    fn put(&self, url: &Url, body: Vec<u8>) -> Result<StatusCode, RemoteProjectionProviderError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("PUT {url} {}", String::from_utf8_lossy(&body)));
        Ok(self.put_status)
    }

    fn propfind(
        &self,
        url: &Url,
        depth: &str,
        max_body_bytes: usize,
    ) -> Result<WebDavHttpResponse, RemoteProjectionProviderError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("PROPFIND {url} depth={depth}"));
        self.propfind_responses
            .lock()
            .expect("propfind")
            .pop_front()
            .ok_or_else(|| RemoteProjectionProviderError::ProviderIo("missing propfind".into()))
            .and_then(|response| limited_response(response, max_body_bytes))
    }

    fn get(
        &self,
        url: &Url,
        max_body_bytes: usize,
    ) -> Result<WebDavHttpResponse, RemoteProjectionProviderError> {
        self.calls.lock().expect("calls").push(format!("GET {url}"));
        self.get_responses
            .lock()
            .expect("get")
            .pop_front()
            .ok_or_else(|| RemoteProjectionProviderError::ProviderIo("missing get".into()))
            .and_then(|response| limited_response(response, max_body_bytes))
    }
}

fn limited_response(
    response: WebDavHttpResponse,
    max_body_bytes: usize,
) -> Result<WebDavHttpResponse, RemoteProjectionProviderError> {
    if response.body.len() > max_body_bytes {
        Err(RemoteProjectionProviderError::ProviderIo(format!(
            "WebDAV response body exceeds {max_body_bytes} bytes"
        )))
    } else {
        Ok(response)
    }
}

#[cfg(unix)]
fn make_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn make_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

fn propfind_body(entries: &[(&str, bool)]) -> String {
    let mut body = String::from(r#"<?xml version="1.0"?><d:multistatus xmlns:d="DAV:">"#);
    for (href, is_collection) in entries {
        body.push_str("<d:response><d:href>");
        body.push_str(href);
        body.push_str("</d:href><d:propstat><d:prop><d:resourcetype>");
        if *is_collection {
            body.push_str("<d:collection/>");
        }
        body.push_str("</d:resourcetype></d:prop></d:propstat></d:response>");
    }
    body.push_str("</d:multistatus>");
    body
}

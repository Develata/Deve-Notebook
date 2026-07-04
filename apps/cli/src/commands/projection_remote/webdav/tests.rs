use super::provider::WebDavTransport;
use super::*;
use deve_core::remote_projection::{
    RemoteProjectionFile, RemoteProjectionProvider, RemoteProjectionProviderAdapter,
    RemoteProjectionProviderError, RemoteProjectionPushRequest,
};
use reqwest::{StatusCode, Url};
use std::fs;
use std::sync::Mutex;

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
}

impl RecordingTransport {
    fn new(mkcol_status: StatusCode, put_status: StatusCode) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            mkcol_status,
            put_status,
        }
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
}

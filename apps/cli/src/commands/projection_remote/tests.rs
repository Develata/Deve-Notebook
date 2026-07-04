use super::*;
use deve_core::remote_projection::{
    RemoteProjectionAuthorityEffects, RemoteProjectionFile, RemoteProjectionProviderError,
    RemoteProjectionPullOutcome, RemoteProjectionPushOutcome,
};
use deve_core::source_control::ChangeStatus;
use std::path::PathBuf;

#[test]
fn webdav_push_builds_provider_request() {
    let request = request_from_action(ProjectionRemoteAction::Webdav {
        action: ProjectionRemoteDirectionAction::Push {
            repo: Some("default".into()),
            locator: "webdav+https://dav.example.com/notebooks/main".into(),
        },
    });

    assert_eq!(request.provider, RemoteProjectionProvider::WebDav);
    assert_eq!(request.direction, RemoteProjectionDirection::Push);
    assert_eq!(request.repo.as_deref(), Some("default"));
}

#[test]
fn s3_pull_builds_provider_request() {
    let request = request_from_action(ProjectionRemoteAction::S3 {
        action: ProjectionRemoteDirectionAction::Pull {
            repo: None,
            locator: "s3://bucket/notebooks/main".into(),
        },
    });

    assert_eq!(request.provider, RemoteProjectionProvider::S3);
    assert_eq!(request.direction, RemoteProjectionDirection::Pull);
    assert_eq!(request.locator, "s3://bucket/notebooks/main");
}

#[test]
fn run_reports_provider_io_fail_closed_after_workspace_gate() {
    let repo = initialized_default_repo();

    let err = run(&repo.ledger_dir(), s3_pull_action(), 8)
        .expect_err("provider I/O must remain fail-closed");

    let message = err.to_string();
    assert!(message.contains("provider I/O is not wired yet"));
    assert!(message.contains("provider_io_ready=false"));
}

#[test]
fn run_checks_workspace_identity_before_provider_io() {
    let repo = initialized_default_repo();
    std::fs::remove_file(deve_core::utils::notegit::repo_identity_path(
        &repo.workspace,
    ))
    .expect("remove identity marker");

    let err = run(&repo.ledger_dir(), s3_pull_action(), 8)
        .expect_err("workspace identity gate must fail before provider I/O");

    let message = err.to_string();
    assert!(message.contains("Projection workspace identity marker is invalid"));
    assert!(message.contains("identity marker"));
    assert!(!message.contains("provider_io_ready=false"));
}

#[test]
fn run_webdav_push_uses_webdav_provider_after_workspace_gate() {
    let repo = initialized_default_repo();
    std::fs::create_dir_all(repo.workspace.join("notes")).expect("notes");
    std::fs::write(repo.workspace.join("notes").join("a.md"), "a").expect("a");
    std::fs::write(repo.workspace.join("scratch.txt"), "skip").expect("txt");
    std::fs::write(repo.workspace.join(".deveignore"), "ignored.md\n").expect("ignore");
    std::fs::write(repo.workspace.join("ignored.md"), "skip").expect("ignored");
    let mut provider = RecordingProvider::default();

    run_with_provider(&repo.ledger_dir(), webdav_push_action(), 8, &mut provider)
        .expect("webdav push");

    assert_eq!(
        provider.uploaded_paths,
        vec![(
            "webdav+https://dav.example.com/notebooks/main".to_string(),
            vec!["notes/a.md".to_string()]
        )]
    );
}

#[test]
fn run_webdav_push_returns_provider_error_before_success_report() {
    let repo = initialized_default_repo();
    std::fs::write(repo.workspace.join("a.md"), "a").expect("a");
    let mut provider = FailingProvider;

    let err = run_with_provider(&repo.ledger_dir(), webdav_push_action(), 8, &mut provider)
        .expect_err("provider failure");

    assert!(err.to_string().contains("simulated WebDAV failure"));
}

#[test]
fn run_webdav_pull_scans_written_files_into_external_changes() {
    let repo = initialized_default_repo();
    let mut provider = PullWritingProvider;

    run_with_provider(&repo.ledger_dir(), webdav_pull_action(), 8, &mut provider)
        .expect("webdav pull");

    let repo_manager =
        deve_core::ledger::RepoManager::init(repo.ledger_dir(), 8, None, None).expect("repo");
    let pending = repo_manager
        .list_pending_fs_in_local_repo("default")
        .expect("pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "remote.md");
    assert_eq!(pending[0].status, ChangeStatus::Added);
    assert!(
        repo_manager
            .list_staged_in_local_repo("default")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn run_webdav_pull_returns_provider_error_before_scan() {
    let repo = initialized_default_repo();
    let mut provider = PullFailingProvider;

    let err = run_with_provider(&repo.ledger_dir(), webdav_pull_action(), 8, &mut provider)
        .expect_err("provider failure");

    assert!(err.to_string().contains("simulated WebDAV pull failure"));
    let repo_manager =
        deve_core::ledger::RepoManager::init(repo.ledger_dir(), 8, None, None).expect("repo");
    assert!(
        repo_manager
            .list_pending_fs_in_local_repo("default")
            .expect("pending")
            .is_empty()
    );
}

#[derive(Default)]
struct RecordingProvider {
    uploaded_paths: Vec<(String, Vec<String>)>,
}

struct FailingProvider;
struct PullWritingProvider;
struct PullFailingProvider;

impl webdav::WebDavProjectionPushAdapter for FailingProvider {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        _locator: &str,
        files: &[webdav::MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        assert_eq!(files.len(), 1);
        Err(RemoteProjectionProviderError::ProviderIo(
            "simulated WebDAV failure".into(),
        ))
    }
}

impl webdav::WebDavProjectionPullAdapter for FailingProvider {
    fn pull_projection_files(
        &self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
        _workspace: &std::path::Path,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        unreachable!("push-only failing provider")
    }
}

impl webdav::WebDavProjectionPushAdapter for RecordingProvider {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        files: &[webdav::MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        self.uploaded_paths.push((
            locator.to_string(),
            files.iter().map(|file| file.path().to_string()).collect(),
        ));
        Ok(RemoteProjectionPushOutcome {
            uploaded_files: files.len(),
            effects:
                deve_core::remote_projection::RemoteProjectionAuthorityEffects::projection_transport(
                ),
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

impl webdav::WebDavProjectionPullAdapter for RecordingProvider {
    fn pull_projection_files(
        &self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
        _workspace: &std::path::Path,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        unreachable!("push-only recording provider")
    }
}

impl webdav::WebDavProjectionPushAdapter for PullWritingProvider {
    fn push_projection_files(
        &mut self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
        _files: &[webdav::MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        unreachable!("pull-only writing provider")
    }
}

impl webdav::WebDavProjectionPullAdapter for PullWritingProvider {
    fn pull_projection_files(
        &self,
        provider: RemoteProjectionProvider,
        _locator: &str,
        workspace: &std::path::Path,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        std::fs::write(workspace.join("remote.md"), "remote").expect("remote file");
        Ok(RemoteProjectionPullOutcome {
            files: vec![RemoteProjectionFile::new("remote.md", b"remote").expect("file")],
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            overwrites_projection_workspace: true,
            external_changes_confirmation_required: true,
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

impl webdav::WebDavProjectionPushAdapter for PullFailingProvider {
    fn push_projection_files(
        &mut self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
        _files: &[webdav::MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        unreachable!("pull-only failing provider")
    }
}

impl webdav::WebDavProjectionPullAdapter for PullFailingProvider {
    fn pull_projection_files(
        &self,
        provider: RemoteProjectionProvider,
        _locator: &str,
        _workspace: &std::path::Path,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        Err(RemoteProjectionProviderError::ProviderIo(
            "simulated WebDAV pull failure".into(),
        ))
    }
}

struct ProjectionRemoteHarness {
    _dir: tempfile::TempDir,
    root: PathBuf,
    workspace: PathBuf,
}

impl ProjectionRemoteHarness {
    fn ledger_dir(&self) -> PathBuf {
        self.root.join("ledger")
    }
}

fn initialized_default_repo() -> ProjectionRemoteHarness {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().to_path_buf();
    crate::commands::init::run(
        &root.join("ledger"),
        "default",
        &root.join("notes"),
        root.clone(),
        8,
        None,
        None,
    )
    .expect("init");
    let workspace = std::fs::read_dir(root.join("notes"))
        .expect("notes dir")
        .map(|entry| entry.expect("workspace entry").path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("default--"))
        })
        .expect("default workspace");

    ProjectionRemoteHarness {
        _dir: dir,
        root,
        workspace,
    }
}

fn s3_pull_action() -> ProjectionRemoteAction {
    ProjectionRemoteAction::S3 {
        action: ProjectionRemoteDirectionAction::Pull {
            repo: Some("default".into()),
            locator: "s3://bucket/notebooks/main".into(),
        },
    }
}

fn webdav_pull_action() -> ProjectionRemoteAction {
    ProjectionRemoteAction::Webdav {
        action: ProjectionRemoteDirectionAction::Pull {
            repo: Some("default".into()),
            locator: "webdav+https://dav.example.com/notebooks/main".into(),
        },
    }
}

fn webdav_push_action() -> ProjectionRemoteAction {
    ProjectionRemoteAction::Webdav {
        action: ProjectionRemoteDirectionAction::Push {
            repo: Some("default".into()),
            locator: "webdav+https://dav.example.com/notebooks/main".into(),
        },
    }
}

use super::*;
use deve_core::source_control::ChangeStatus;

mod support;
use support::*;

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

    let message = err.to_string();
    assert!(message.contains("provider_io_ready=false"));
    assert!(message.contains("simulated WebDAV failure"));
}

#[test]
fn run_s3_push_uses_s3_provider_after_workspace_gate() {
    let repo = initialized_default_repo();
    std::fs::create_dir_all(repo.workspace.join("notes")).expect("notes");
    std::fs::write(repo.workspace.join("notes").join("a.md"), "a").expect("a");
    for reserved in [
        ".git",
        ".notegit",
        "ledger",
        "snapshot",
        "snapshots",
        "staging",
    ] {
        std::fs::create_dir_all(repo.workspace.join(reserved)).expect("reserved dir");
        std::fs::write(repo.workspace.join(reserved).join("leak.md"), "leak").expect("leak");
    }
    let mut webdav_provider = RecordingProvider::default();
    let mut s3_provider = RecordingS3Provider::default();

    run_with_providers(
        &repo.ledger_dir(),
        s3_push_action(),
        8,
        &mut webdav_provider,
        &mut s3_provider,
    )
    .expect("s3 push");

    assert_eq!(
        webdav_provider.uploaded_paths,
        Vec::<(String, Vec<String>)>::new()
    );
    assert_eq!(
        s3_provider.uploaded_paths,
        vec![(
            "s3://bucket/notebooks/main".to_string(),
            vec!["notes/a.md".to_string()]
        )]
    );
}

#[test]
fn run_s3_push_checks_workspace_identity_before_provider_io() {
    let repo = initialized_default_repo();
    std::fs::write(repo.workspace.join("a.md"), "a").expect("a");
    std::fs::remove_file(deve_core::utils::notegit::repo_identity_path(
        &repo.workspace,
    ))
    .expect("remove identity marker");
    let mut webdav_provider = RecordingProvider::default();
    let mut s3_provider = RecordingS3Provider::default();

    let err = run_with_providers(
        &repo.ledger_dir(),
        s3_push_action(),
        8,
        &mut webdav_provider,
        &mut s3_provider,
    )
    .expect_err("workspace identity gate must fail before S3 provider I/O");

    let message = err.to_string();
    assert!(message.contains("Projection workspace identity marker is invalid"));
    assert!(s3_provider.uploaded_paths.is_empty());
}

#[test]
fn run_s3_pull_scans_written_files_into_external_changes() {
    let repo = initialized_default_repo();
    let mut webdav_provider = RecordingProvider::default();
    let mut s3_provider = S3PullWritingProvider;

    run_with_providers(
        &repo.ledger_dir(),
        s3_pull_action(),
        8,
        &mut webdav_provider,
        &mut s3_provider,
    )
    .expect("s3 pull");

    let repo_manager =
        deve_core::ledger::RepoManager::init(repo.ledger_dir(), 8, None, None).expect("repo");
    let pending = repo_manager
        .list_pending_fs_in_local_repo("default")
        .expect("pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "remote-s3.md");
    assert_eq!(pending[0].status, ChangeStatus::Added);
    assert!(
        repo_manager
            .list_staged_in_local_repo("default")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn run_s3_pull_returns_provider_error_before_scan() {
    let repo = initialized_default_repo();
    let mut webdav_provider = RecordingProvider::default();
    let mut s3_provider = S3PullFailingProvider;

    let err = run_with_providers(
        &repo.ledger_dir(),
        s3_pull_action(),
        8,
        &mut webdav_provider,
        &mut s3_provider,
    )
    .expect_err("provider failure");

    let message = err.to_string();
    assert!(message.contains("provider_io_ready=false"));
    assert!(message.contains("simulated S3 pull failure"));
    let repo_manager =
        deve_core::ledger::RepoManager::init(repo.ledger_dir(), 8, None, None).expect("repo");
    assert!(
        repo_manager
            .list_pending_fs_in_local_repo("default")
            .expect("pending")
            .is_empty()
    );
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

    let message = err.to_string();
    assert!(message.contains("provider_io_ready=false"));
    assert!(message.contains("simulated WebDAV pull failure"));
    let repo_manager =
        deve_core::ledger::RepoManager::init(repo.ledger_dir(), 8, None, None).expect("repo");
    assert!(
        repo_manager
            .list_pending_fs_in_local_repo("default")
            .expect("pending")
            .is_empty()
    );
}

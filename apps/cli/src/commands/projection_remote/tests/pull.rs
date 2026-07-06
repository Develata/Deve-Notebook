//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::super::{run_with_provider, run_with_providers};
use super::support::{
    PullDuplicatePathProvider, PullFailingProvider, PullWithoutExternalChangesProvider,
    PullWithoutWorkspaceOverwriteProvider, PullWritingProvider, RecordingProvider,
    S3PullFailingProvider, S3PullWritingProvider, initialized_default_repo, s3_pull_action,
    webdav_pull_action,
};
use deve_core::source_control::ChangeStatus;

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

#[test]
fn run_rejects_duplicate_pull_paths_before_workspace_write() {
    let repo = initialized_default_repo();
    std::fs::write(repo.workspace.join("remote-duplicate.md"), "local sentinel")
        .expect("local sentinel");
    let mut provider = PullDuplicatePathProvider;

    let err = run_with_provider(&repo.ledger_dir(), webdav_pull_action(), 8, &mut provider)
        .expect_err("duplicate pull paths must fail closed");

    let message = err.to_string();
    assert!(message.contains("provider_io_ready=false"));
    assert!(message.contains("duplicate projection path remote-duplicate.md"));
    assert_eq!(
        std::fs::read_to_string(repo.workspace.join("remote-duplicate.md")).expect("sentinel"),
        "local sentinel"
    );
    let repo_manager =
        deve_core::ledger::RepoManager::init(repo.ledger_dir(), 8, None, None).expect("repo");
    assert!(
        repo_manager
            .list_pending_fs_in_local_repo("default")
            .expect("pending")
            .is_empty()
    );
    assert!(
        repo_manager
            .list_staged_in_local_repo("default")
            .expect("staged")
            .is_empty()
    );
}

#[test]
fn run_rejects_pull_without_external_changes_confirmation_before_workspace_write() {
    let repo = initialized_default_repo();
    let mut provider = PullWithoutExternalChangesProvider;

    let err = run_with_provider(&repo.ledger_dir(), webdav_pull_action(), 8, &mut provider)
        .expect_err("pull without External Changes confirmation must fail closed");

    let message = err.to_string();
    assert!(message.contains("provider_io_ready=false"));
    assert!(message.contains("pull must require External Changes confirmation"));
    assert!(!repo.workspace.join("remote-unconfirmed.md").exists());
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
fn run_rejects_pull_without_projection_workspace_overwrite_before_workspace_write() {
    let repo = initialized_default_repo();
    let mut provider = PullWithoutWorkspaceOverwriteProvider;

    let err = run_with_provider(&repo.ledger_dir(), webdav_pull_action(), 8, &mut provider)
        .expect_err("pull without Projection Workspace overwrite must fail closed");

    let message = err.to_string();
    assert!(message.contains("provider_io_ready=false"));
    assert!(message.contains("pull must overwrite only the Projection Workspace"));
    assert!(!repo.workspace.join("remote-no-overwrite.md").exists());
    let repo_manager =
        deve_core::ledger::RepoManager::init(repo.ledger_dir(), 8, None, None).expect("repo");
    assert!(
        repo_manager
            .list_pending_fs_in_local_repo("default")
            .expect("pending")
            .is_empty()
    );
}

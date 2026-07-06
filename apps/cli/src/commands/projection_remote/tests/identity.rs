//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::super::{run, run_with_providers};
use super::support::{
    RecordingProvider, RecordingS3Provider, initialized_default_repo, s3_pull_action,
    s3_push_action,
};

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

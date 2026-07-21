//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::super::run_with_providers;
use super::support::{
    RecordingProvider, RecordingS3Provider, initialized_default_repo, s3_push_action,
};

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
    assert!(
        message.contains("identity marker"),
        "unexpected identity admission error: {message}"
    );
    assert!(s3_provider.uploaded_paths.is_empty());
}

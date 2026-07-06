//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::super::{run_with_provider, run_with_providers};
use super::support::{
    AuthoritativeMetadataPushProvider, AuthorityEffectPushProvider, FailingProvider,
    RecordingProvider, RecordingS3Provider, initialized_default_repo, s3_push_action,
    webdav_push_action,
};

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
fn run_rejects_provider_authority_effects_before_success_report() {
    let repo = initialized_default_repo();
    std::fs::write(repo.workspace.join("a.md"), "a").expect("a");
    let mut provider = AuthorityEffectPushProvider;

    let err = run_with_provider(&repo.ledger_dir(), webdav_push_action(), 8, &mut provider)
        .expect_err("provider authority effects must fail closed");

    let message = err.to_string();
    assert!(message.contains("provider_io_ready=false"));
    assert!(message.contains("authority effects must be absent"));
    assert!(message.contains("writes_ledger=true"));
}

#[test]
fn run_rejects_authoritative_provider_metadata_before_success_report() {
    let repo = initialized_default_repo();
    std::fs::write(repo.workspace.join("a.md"), "a").expect("a");
    let mut provider = AuthoritativeMetadataPushProvider;

    let err = run_with_provider(&repo.ledger_dir(), webdav_push_action(), 8, &mut provider)
        .expect_err("provider authoritative metadata must fail closed");

    let message = err.to_string();
    assert!(message.contains("provider_io_ready=false"));
    assert!(message.contains("provider metadata must be diagnostic-only"));
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

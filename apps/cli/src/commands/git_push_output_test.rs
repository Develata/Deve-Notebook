use super::{print_push_report, push_report_lines};
use deve_core::git_bridge::{GitMirrorPushBlocker, GitMirrorPushReport};

#[test]
fn print_git_push_report_handles_pushed_head() {
    print_push_report(
        "default",
        &GitMirrorPushReport {
            remote: Some("origin".into()),
            branch: Some("main".into()),
            remote_url: Some("git@example.invalid:repo.git".into()),
            head: Some("abc123".into()),
            pushed: true,
            blockers: Vec::new(),
        },
    );
}

#[test]
fn push_report_lines_keep_git_as_mirror_only() {
    let pushed = push_report_lines(
        "default",
        &GitMirrorPushReport {
            remote: Some("origin".into()),
            branch: Some("main".into()),
            remote_url: Some("git@example.invalid:repo.git".into()),
            head: Some("abc123".into()),
            pushed: true,
            blockers: Vec::new(),
        },
    );

    assert!(
        pushed
            .iter()
            .any(|line| line.contains("git_push[default]: pushed=true remote=origin branch=main"))
    );
    assert!(
        pushed
            .iter()
            .any(|line| line.contains("Deve ledger remains authority"))
    );

    let blocked = push_report_lines(
        "default",
        &GitMirrorPushReport {
            remote: Some("origin".into()),
            branch: Some("main".into()),
            remote_url: None,
            head: Some("abc123".into()),
            pushed: false,
            blockers: vec![GitMirrorPushBlocker {
                location: "git_history_mapping".into(),
                reason: "unpublished mirror records".into(),
            }],
        },
    );

    assert!(
        blocked
            .iter()
            .any(|line| line.contains("no remote push was performed"))
    );
    assert!(
        blocked
            .iter()
            .any(|line| line.contains("location=git_history_mapping"))
    );
}

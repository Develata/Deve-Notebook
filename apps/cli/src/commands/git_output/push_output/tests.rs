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

    assert_eq!(
        pushed,
        vec![
            "git_push[default]: pushed=true remote=origin branch=main head=abc123 blockers=0",
            "  remote_url: git@example.invalid:repo.git",
            "  push_hint: Git mirror HEAD was pushed; Deve ledger remains authority",
        ]
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

    assert_eq!(
        blocked,
        vec![
            "git_push[default]: pushed=false remote=origin branch=main head=abc123 blockers=1",
            "  blocker[1]: location=git_history_mapping reason=unpublished mirror records",
            "    hint: run `deve_cli git export --repo default` or `deve_cli git export --repo default --retry-out-of-sync` so Git HEAD maps to latest Deve commit",
            "  push_hint: no remote push was performed; fix the blocker hint(s) above first",
        ]
    );
}

#[test]
fn push_report_lines_explain_remote_and_worktree_blockers() {
    let lines = push_report_lines(
        "default",
        &GitMirrorPushReport {
            remote: None,
            branch: Some("main".into()),
            remote_url: None,
            head: Some("abc123".into()),
            pushed: false,
            blockers: vec![
                GitMirrorPushBlocker {
                    location: "git_remote".into(),
                    reason: "remote origin not configured".into(),
                },
                GitMirrorPushBlocker {
                    location: "git_worktree".into(),
                    reason: "worktree has changes".into(),
                },
            ],
        },
    );

    assert_eq!(
        lines,
        vec![
            "git_push[default]: pushed=false remote=- branch=main head=abc123 blockers=2",
            "  blocker[1]: location=git_remote reason=remote origin not configured",
            "    hint: configure branch upstream/origin, or pass `--remote <remote> --branch <branch>`",
            "  blocker[2]: location=git_worktree reason=worktree has changes",
            "    hint: clean Git worktree or run `deve_cli git import --apply --repo default` before pushing",
            "  push_hint: no remote push was performed; fix the blocker hint(s) above first",
        ]
    );
}

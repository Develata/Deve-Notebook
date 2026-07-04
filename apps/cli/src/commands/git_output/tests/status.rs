use super::super::{print_status, status_lines_at};
use super::support::record;
use deve_core::git_bridge::{
    GitMetadataKind, GitMirrorCommitState, GitMirrorRecord, GitMirrorState, GitMirrorStatus,
    GitMirrorSummary,
};

#[test]
fn print_git_status_handles_disabled_repo() {
    print_status(
        "default",
        &GitMirrorStatus {
            repo_root: std::path::PathBuf::from("notes/default"),
            notegit_present: true,
            git_metadata_kind: GitMetadataKind::Missing,
            gitignore_protects_notegit: true,
            state: GitMirrorState::Disabled,
            reason: None,
        },
        &GitMirrorSummary::default(),
        &[],
    );
}

#[test]
fn print_git_status_handles_out_of_sync_summary() {
    print_status(
        "default",
        &GitMirrorStatus {
            repo_root: std::path::PathBuf::from("notes/default"),
            notegit_present: true,
            git_metadata_kind: GitMetadataKind::Directory,
            gitignore_protects_notegit: true,
            state: GitMirrorState::Ready,
            reason: None,
        },
        &GitMirrorSummary {
            queued: 1,
            committed: 2,
            out_of_sync: 1,
        },
        &[
            record("deve-1", GitMirrorCommitState::Queued, 7, None),
            record(
                "deve-2",
                GitMirrorCommitState::OutOfSync,
                8,
                Some("Git mirror refuses to run with 1 pending source-control change(s)"),
            ),
        ],
    );
}

#[test]
fn status_lines_include_per_commit_lag_and_retry_hint() {
    let lines = status_lines_at(
        "default",
        &GitMirrorStatus {
            repo_root: std::path::PathBuf::from("notes/default"),
            notegit_present: true,
            git_metadata_kind: GitMetadataKind::Directory,
            gitignore_protects_notegit: true,
            state: GitMirrorState::Ready,
            reason: None,
        },
        &GitMirrorSummary {
            queued: 1,
            committed: 0,
            out_of_sync: 1,
        },
        &[
            record("deve-1", GitMirrorCommitState::Queued, 7, None),
            record(
                "deve-2",
                GitMirrorCommitState::OutOfSync,
                8,
                Some("Git mirror refuses to run with 1 pending source-control change(s)"),
            ),
        ],
        11,
    );

    assert!(lines.iter().any(|line| line.contains("lagging_records=2")));
    assert!(
        lines
            .iter()
            .any(|line| line.contains("lag[1]: deve_commit=deve-1 state=queued ledger_seq=7"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("queued_lag_ms=10 updated_lag_ms=9"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("failure[2]: location=deve_source_control"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("--retry-out-of-sync"))
    );
}

#[test]
fn status_lines_keep_ordered_lagging_output_contract() {
    let lines = status_lines_at(
        "default",
        &GitMirrorStatus {
            repo_root: std::path::PathBuf::from("notes/default"),
            notegit_present: true,
            git_metadata_kind: GitMetadataKind::Directory,
            gitignore_protects_notegit: true,
            state: GitMirrorState::Ready,
            reason: None,
        },
        &GitMirrorSummary {
            queued: 1,
            committed: 0,
            out_of_sync: 0,
        },
        &[record("deve-1", GitMirrorCommitState::Queued, 7, None)],
        11,
    );

    assert_eq!(
        lines,
        vec![
            "git_status[default]: state=ready queue_state=queued git=directory notegit=present gitignore_notegit=protected queued=1 committed=0 out_of_sync=0",
            "  repo_root: notes/default",
            "  lagging_records=1",
            "  lag[1]: deve_commit=deve-1 state=queued ledger_seq=7 attempts=1 git_commit=- queued_lag_ms=10 updated_lag_ms=9 queued_at_ms=1 updated_at_ms=2",
            "  next_action: run `deve_cli ngit mirror --repo default` to mirror queued Deve commit(s)",
        ]
    );
}

#[test]
fn status_lines_include_git_mirror_failure_metadata() {
    let mut failed = record(
        "deve-1",
        GitMirrorCommitState::OutOfSync,
        7,
        Some("git commit failed (status exit status: 128): missing user.name"),
    );
    failed.failure_command = Some("commit".to_string());
    failed.failure_exit_status = Some("exit status: 128".to_string());

    let lines = status_lines_at(
        "default",
        &GitMirrorStatus {
            repo_root: std::path::PathBuf::from("notes/default"),
            notegit_present: true,
            git_metadata_kind: GitMetadataKind::Directory,
            gitignore_protects_notegit: true,
            state: GitMirrorState::Ready,
            reason: None,
        },
        &GitMirrorSummary {
            queued: 0,
            committed: 0,
            out_of_sync: 1,
        },
        &[failed],
        11,
    );

    assert!(lines.iter().any(|line| {
        line.contains("failure_meta[1]: subject=- command=commit exit_status=exit status: 128")
    }));
}

#[test]
fn status_lines_fallback_classifies_legacy_records_without_failure_stage() {
    let lines = status_lines_at(
        "default",
        &GitMirrorStatus {
            repo_root: std::path::PathBuf::from("notes/default"),
            notegit_present: true,
            git_metadata_kind: GitMetadataKind::Directory,
            gitignore_protects_notegit: true,
            state: GitMirrorState::Ready,
            reason: None,
        },
        &GitMirrorSummary {
            queued: 0,
            committed: 0,
            out_of_sync: 1,
        },
        &[GitMirrorRecord {
            failure_stage: None,
            ..record(
                "deve-legacy",
                GitMirrorCommitState::OutOfSync,
                9,
                Some("Git mirror refuses unsafe projection path: .notegit/state"),
            )
        }],
        11,
    );

    assert!(
        lines
            .iter()
            .any(|line| line.contains("failure[1]: location=notegit_protection"))
    );
}

#[test]
fn status_lines_legacy_fallback_uses_core_failure_stage_classifier() {
    let lines = status_lines_at(
        "default",
        &GitMirrorStatus {
            repo_root: std::path::PathBuf::from("notes/default"),
            notegit_present: true,
            git_metadata_kind: GitMetadataKind::Directory,
            gitignore_protects_notegit: true,
            state: GitMirrorState::Ready,
            reason: None,
        },
        &GitMirrorSummary {
            queued: 0,
            committed: 0,
            out_of_sync: 1,
        },
        &[GitMirrorRecord {
            failure_stage: None,
            ..record(
                "deve-legacy",
                GitMirrorCommitState::OutOfSync,
                9,
                Some(
                    "Git mirror refuses to include path(s) outside queued Deve commit: outside.md, .notegit/state",
                ),
            )
        }],
        11,
    );

    assert!(
        lines
            .iter()
            .any(|line| line.contains("failure[1]: location=projection_scope"))
    );
}

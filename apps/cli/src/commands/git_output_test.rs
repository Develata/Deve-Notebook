use super::{mirror_report_lines, print_mirror_report, print_status, status_lines_at};
use deve_core::git_bridge::{
    GitMetadataKind, GitMirrorCommitState, GitMirrorFailureStage, GitMirrorRecord,
    GitMirrorRunReport, GitMirrorState, GitMirrorStatus, GitMirrorSummary,
};

fn record(
    id: &str,
    state: GitMirrorCommitState,
    ledger_seq: u64,
    last_error: Option<&str>,
) -> GitMirrorRecord {
    GitMirrorRecord {
        deve_commit_id: id.into(),
        repo_id: uuid::Uuid::nil(),
        ledger_seq,
        state,
        git_commit_id: None,
        last_error: last_error.map(str::to_string),
        failure_stage: last_error.map(GitMirrorFailureStage::classify),
        queued_at_ms: 1,
        updated_at_ms: 2,
        attempts: 1,
    }
}

#[test]
fn print_git_status_handles_disabled_repo() {
    print_status(
        "default",
        &GitMirrorStatus {
            repo_root: std::path::PathBuf::from("vault/default"),
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
            repo_root: std::path::PathBuf::from("vault/default"),
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
            repo_root: std::path::PathBuf::from("vault/default"),
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
fn status_lines_fallback_classifies_legacy_records_without_failure_stage() {
    let lines = status_lines_at(
        "default",
        &GitMirrorStatus {
            repo_root: std::path::PathBuf::from("vault/default"),
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
fn print_git_mirror_report_handles_committed_record() {
    print_mirror_report(
        "default",
        &GitMirrorRunReport {
            attempted: 1,
            committed: 1,
            out_of_sync: 0,
            skipped: 0,
            records: vec![GitMirrorRecord {
                git_commit_id: Some("abc123".into()),
                ..record("deve-1", GitMirrorCommitState::Committed, 7, None)
            }],
        },
    );
}

#[test]
fn mirror_report_lines_include_noop_and_repair_semantics() {
    let empty = mirror_report_lines("default", &GitMirrorRunReport::default());
    assert!(
        empty
            .iter()
            .any(|line| line.contains("no queued Git mirror records"))
    );

    let failed = mirror_report_lines(
        "default",
        &GitMirrorRunReport {
            attempted: 1,
            committed: 0,
            out_of_sync: 1,
            skipped: 0,
            records: vec![record(
                "deve-1",
                GitMirrorCommitState::OutOfSync,
                7,
                Some("Git mirror refuses unsafe projection path: .notegit/state"),
            )],
        },
    );

    assert!(
        failed
            .iter()
            .any(|line| line.contains("location=notegit_protection"))
    );
    assert!(
        failed
            .iter()
            .any(|line| line.contains("--retry-out-of-sync"))
    );
}

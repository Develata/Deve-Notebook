use super::super::{
    export_report_lines, mirror_report_lines, print_export_report, print_mirror_report,
};
use super::support::record;
use deve_core::git_bridge::{GitMirrorCommitState, GitMirrorRecord, GitMirrorRunReport};

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
fn print_git_export_report_handles_committed_record() {
    print_export_report(
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

#[test]
fn mirror_report_lines_keep_ordered_success_contract() {
    let lines = mirror_report_lines(
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

    assert_eq!(
        lines,
        vec![
            "git_mirror[default]: attempted=1 committed=1 out_of_sync=0 skipped=0",
            "  record[1]: deve_commit=deve-1 state=committed ledger_seq=7 attempts=1 git_commit=abc123 queued_at_ms=1 updated_at_ms=2",
            "  mirror_hint: attempted records are mirrored",
        ]
    );
}

#[test]
fn export_report_lines_use_export_semantics_and_retry_command() {
    let empty = export_report_lines("default", &GitMirrorRunReport::default());
    assert_eq!(
        empty,
        vec![
            "git_export[default]: attempted=0 committed=0 out_of_sync=0 skipped=0",
            "  export_hint: no queued Git mirror records to export",
        ]
    );

    let failed = export_report_lines(
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
                Some("Git mirror refuses to run with 1 pending source-control change(s)"),
            )],
        },
    );

    assert_eq!(
        failed,
        vec![
            "git_export[default]: attempted=1 committed=0 out_of_sync=1 skipped=0",
            "  record[1]: deve_commit=deve-1 state=out_of_sync ledger_seq=7 attempts=1 git_commit=- queued_at_ms=1 updated_at_ms=2",
            "  failure[1]: location=deve_source_control error=Git mirror refuses to run with 1 pending source-control change(s)",
            "  repair_action[1]: code=clean_deve_source_control retryable_after_fix=yes subject=deve_source_control",
            "  repair_guidance[1]: manual_only=yes next=stage_commit_or_discard_deve_source_control_changes retry_command=`deve_cli git export --repo default --retry-out-of-sync`",
            "  repair_hint: fix the reported failure_location/error; retry with `deve_cli git export --repo default --retry-out-of-sync`",
        ]
    );
}

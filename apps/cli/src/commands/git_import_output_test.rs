use super::{import_apply_report_lines, import_plan_lines};
use deve_core::git_bridge::{
    GitImportApplyReport, GitImportPlan, GitImportPlanBlocker, GitImportPlanEntry,
};
use deve_core::source_control::ChangeStatus;

#[test]
fn import_plan_lines_are_explicitly_dry_run_and_non_authoritative() {
    let clean = import_plan_lines(
        "default",
        &GitImportPlan {
            repo_root: std::path::PathBuf::from("vault/default"),
            entries: vec![GitImportPlanEntry {
                path: "note.md".into(),
                previous_path: None,
                status: ChangeStatus::Modified,
                git_status: "M".into(),
            }],
            blockers: Vec::new(),
        },
    );

    assert!(
        clean
            .iter()
            .any(|line| line.contains("git_import[default]: dry_run=true changes=1 blockers=0"))
    );
    assert!(
        clean
            .iter()
            .any(|line| line.contains("rerun with --apply to write pending/import, not ledger"))
    );

    let blocked = import_plan_lines(
        "default",
        &GitImportPlan {
            repo_root: std::path::PathBuf::from("vault/default"),
            entries: Vec::new(),
            blockers: vec![GitImportPlanBlocker {
                path: ".git/config".into(),
                reason: "Git import refuses unsafe path: .git/config".into(),
            }],
        },
    );

    assert!(
        blocked
            .iter()
            .any(|line| line.contains("fix blockers before rerunning with --apply"))
    );
}

#[test]
fn import_apply_report_lines_point_back_to_deve_source_control() {
    let applied = import_apply_report_lines(
        "default",
        &GitImportApplyReport {
            plan: GitImportPlan {
                repo_root: std::path::PathBuf::from("vault/default"),
                entries: vec![GitImportPlanEntry {
                    path: "note.md".into(),
                    previous_path: None,
                    status: ChangeStatus::Modified,
                    git_status: "M".into(),
                }],
                blockers: Vec::new(),
            },
            applied: 1,
            skipped: 0,
            blockers: Vec::new(),
        },
    );

    assert!(
        applied
            .iter()
            .any(|line| line.contains("git_import_apply[default]: applied=1 skipped=0"))
    );
    assert!(
        applied
            .iter()
            .any(|line| line.contains("stage and commit through Deve source control"))
    );

    let blocked = import_apply_report_lines(
        "default",
        &GitImportApplyReport {
            plan: GitImportPlan {
                repo_root: std::path::PathBuf::from("vault/default"),
                entries: Vec::new(),
                blockers: Vec::new(),
            },
            applied: 0,
            skipped: 0,
            blockers: vec![GitImportPlanBlocker {
                path: "-".into(),
                reason: "source-control staged".into(),
            }],
        },
    );

    assert!(
        blocked
            .iter()
            .any(|line| line.contains("no pending/import writes were performed"))
    );
}

#[test]
fn import_apply_report_lines_count_plan_blockers_once() {
    let lines = import_apply_report_lines(
        "default",
        &GitImportApplyReport {
            plan: GitImportPlan {
                repo_root: std::path::PathBuf::from("vault/default"),
                entries: Vec::new(),
                blockers: vec![GitImportPlanBlocker {
                    path: "bad.md".into(),
                    reason: "unsupported Git status T".into(),
                }],
            },
            applied: 0,
            skipped: 0,
            blockers: Vec::new(),
        },
    );

    assert!(
        lines.iter().any(|line| line
            .contains("git_import_apply[default]: applied=0 skipped=0 changes=0 blockers=1")),
        "{lines:?}"
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.contains("unsupported Git status T"))
            .count(),
        1
    );
}

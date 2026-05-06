use super::super::{print_import_apply_report, print_import_plan};
use deve_core::git_bridge::{
    GitImportApplyReport, GitImportPlan, GitImportPlanBlocker, GitImportPlanEntry,
};
use deve_core::source_control::ChangeStatus;

#[test]
fn print_git_import_plan_handles_dry_run_changes() {
    print_import_plan(
        "default",
        &GitImportPlan {
            repo_root: std::path::PathBuf::from("vault/default"),
            entries: vec![GitImportPlanEntry {
                path: "renamed.md".into(),
                previous_path: Some("old.md".into()),
                status: ChangeStatus::Renamed,
                git_status: "R100".into(),
            }],
            blockers: vec![GitImportPlanBlocker {
                path: ".notegit/state".into(),
                reason: "Git import refuses unsafe path: .notegit/state".into(),
            }],
        },
    );
}

#[test]
fn print_git_import_apply_report_handles_pending_import() {
    print_import_apply_report(
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
}

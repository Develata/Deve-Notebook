//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!   - 12_commands#cli-commands
//!
//! Human-readable Git import dry-run and apply diagnostics.

use deve_core::git_bridge::{GitImportApplyReport, GitImportPlan, GitImportPlanBlocker};
use deve_core::source_control::ChangeStatus;

pub(super) fn import_plan_lines(repo_name: &str, plan: &GitImportPlan) -> Vec<String> {
    let mut lines = vec![format!(
        "git_import[{repo_name}]: dry_run=true changes={} blockers={}",
        plan.entries.len(),
        plan.blockers.len()
    )];
    lines.push(format!("  repo_root: {}", plan.repo_root.display()));
    for (index, entry) in plan.entries.iter().enumerate() {
        lines.push(format!(
            "  change[{}]: status={} path={} previous_path={} git_status={}",
            index + 1,
            change_status_label(entry.status),
            entry.path,
            entry.previous_path.as_deref().unwrap_or("-"),
            entry.git_status
        ));
    }
    for (index, blocker) in plan.blockers.iter().enumerate() {
        lines.push(format!(
            "  blocker[{}]: path={} reason={}",
            index + 1,
            blocker.path,
            blocker.reason
        ));
    }
    if plan.entries.is_empty() && plan.blockers.is_empty() {
        lines.push("  import_hint: no Git worktree changes to import".to_string());
    } else if plan.blockers.is_empty() {
        lines.push(
            "  import_hint: dry-run only; rerun with --apply to write pending/import, not ledger"
                .to_string(),
        );
    } else {
        lines.push("  import_hint: fix blockers before rerunning with --apply".to_string());
    }
    lines
}

pub(super) fn import_apply_report_lines(
    repo_name: &str,
    report: &GitImportApplyReport,
) -> Vec<String> {
    let blockers = import_apply_blockers(report);
    let mut lines = vec![format!(
        "git_import_apply[{repo_name}]: applied={} skipped={} changes={} blockers={}",
        report.applied,
        report.skipped,
        report.plan.entries.len(),
        blockers.len()
    )];
    lines.push(format!("  repo_root: {}", report.plan.repo_root.display()));
    for (index, entry) in report.plan.entries.iter().enumerate() {
        lines.push(format!(
            "  change[{}]: status={} path={} previous_path={} git_status={}",
            index + 1,
            change_status_label(entry.status),
            entry.path,
            entry.previous_path.as_deref().unwrap_or("-"),
            entry.git_status
        ));
    }
    for (index, blocker) in blockers.iter().enumerate() {
        lines.push(format!(
            "  blocker[{}]: path={} reason={}",
            index + 1,
            blocker.path,
            blocker.reason
        ));
    }
    if report.plan.entries.is_empty() && blockers.is_empty() {
        lines.push("  apply_hint: no Git worktree changes to import".to_string());
    } else if blockers.is_empty() {
        lines.push(
            "  apply_hint: imported changes are now pending; stage and commit through Deve source control"
                .to_string(),
        );
    } else {
        lines.push(
            "  apply_hint: no pending/import writes were performed; fix blockers".to_string(),
        );
    }
    lines
}

fn import_apply_blockers(report: &GitImportApplyReport) -> Vec<GitImportPlanBlocker> {
    let mut blockers = report.plan.blockers.clone();
    blockers.extend(report.blockers.clone());
    blockers
}

fn change_status_label(status: ChangeStatus) -> &'static str {
    match status {
        ChangeStatus::Added => "added",
        ChangeStatus::Modified => "modified",
        ChangeStatus::Deleted => "deleted",
        ChangeStatus::Renamed => "renamed",
    }
}

#[cfg(test)]
mod tests;

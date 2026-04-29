//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!   - 12_commands#cli-commands
//!
//! Human-readable Git mirror diagnostics.

use deve_core::git_bridge::{
    GitImportApplyReport, GitImportPlan, GitImportPlanBlocker, GitMirrorCommitState,
    GitMirrorRecord, GitMirrorRepairAction, GitMirrorRepairActionCode, GitMirrorRunReport,
    GitMirrorStatus, GitMirrorSummary,
};
use deve_core::source_control::ChangeStatus;

#[path = "git_push_output.rs"]
mod push_output;

pub(super) use push_output::print_push_report;

pub(super) fn print_status(
    repo_name: &str,
    status: &GitMirrorStatus,
    summary: &GitMirrorSummary,
    records: &[GitMirrorRecord],
) {
    for line in status_lines(repo_name, status, summary, records) {
        println!("{line}");
    }
}

pub(super) fn print_mirror_report(repo_name: &str, report: &GitMirrorRunReport) {
    for line in mirror_report_lines(repo_name, report) {
        println!("{line}");
    }
}

pub(super) fn print_export_report(repo_name: &str, report: &GitMirrorRunReport) {
    for line in export_report_lines(repo_name, report) {
        println!("{line}");
    }
}

pub(super) fn print_import_plan(repo_name: &str, plan: &GitImportPlan) {
    for line in import_plan_lines(repo_name, plan) {
        println!("{line}");
    }
}

pub(super) fn print_import_apply_report(repo_name: &str, report: &GitImportApplyReport) {
    for line in import_apply_report_lines(repo_name, report) {
        println!("{line}");
    }
}

fn status_lines(
    repo_name: &str,
    status: &GitMirrorStatus,
    summary: &GitMirrorSummary,
    records: &[GitMirrorRecord],
) -> Vec<String> {
    status_lines_at(
        repo_name,
        status,
        summary,
        records,
        chrono::Utc::now().timestamp_millis(),
    )
}

fn status_lines_at(
    repo_name: &str,
    status: &GitMirrorStatus,
    summary: &GitMirrorSummary,
    records: &[GitMirrorRecord],
    now_ms: i64,
) -> Vec<String> {
    let mut lines = vec![format!(
        "git_status[{repo_name}]: state={} queue_state={} git={} notegit={} gitignore_notegit={} queued={} committed={} out_of_sync={}",
        status.state.as_str(),
        queue_state_label(summary),
        status.git_metadata_kind.as_str(),
        bool_label(status.notegit_present),
        protection_label(status.gitignore_protects_notegit),
        summary.queued,
        summary.committed,
        summary.out_of_sync
    )];
    if let Some(reason) = &status.reason {
        lines.push(format!("  reason: {reason}"));
    }
    lines.push(format!("  repo_root: {}", status.repo_root.display()));

    let lagging: Vec<_> = records
        .iter()
        .filter(|record| {
            matches!(
                record.state,
                GitMirrorCommitState::Queued | GitMirrorCommitState::OutOfSync
            )
        })
        .collect();
    if lagging.is_empty() {
        lines.push("  lagging_records=0".to_string());
        return lines;
    }

    lines.push(format!("  lagging_records={}", lagging.len()));
    let retry_command = git_command("export", repo_name, true);
    for (index, record) in lagging.into_iter().enumerate() {
        lines.extend(record_detail_lines(
            "lag",
            index + 1,
            record,
            Some(now_ms),
            Some(retry_command.as_str()),
        ));
    }

    if summary.queued > 0 {
        lines.push(format!(
            "  next_action: run `{}` to mirror queued Deve commit(s)",
            mirror_command(repo_name, false)
        ));
    }
    if summary.out_of_sync > 0 {
        lines.push(format!(
            "  repair_hint: fix the reported repair_action subject, then run `{}`",
            retry_command
        ));
    }
    lines
}

fn mirror_report_lines(repo_name: &str, report: &GitMirrorRunReport) -> Vec<String> {
    run_report_lines(
        repo_name,
        report,
        RunReportCopy {
            header: "git_mirror",
            hint_label: "mirror_hint",
            no_records_hint: "no queued Git mirror records",
            success_hint: "attempted records are mirrored",
            retry_action: "mirror",
        },
    )
}

fn export_report_lines(repo_name: &str, report: &GitMirrorRunReport) -> Vec<String> {
    run_report_lines(
        repo_name,
        report,
        RunReportCopy {
            header: "git_export",
            hint_label: "export_hint",
            no_records_hint: "no queued Git mirror records to export",
            success_hint: "attempted records are exported to Git mirror",
            retry_action: "export",
        },
    )
}

fn import_plan_lines(repo_name: &str, plan: &GitImportPlan) -> Vec<String> {
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
            "  import_hint: dry-run only; future apply will write pending/import, not ledger"
                .to_string(),
        );
    } else {
        lines.push("  import_hint: fix blockers before future import apply".to_string());
    }
    lines
}

fn import_apply_report_lines(repo_name: &str, report: &GitImportApplyReport) -> Vec<String> {
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

struct RunReportCopy {
    header: &'static str,
    hint_label: &'static str,
    no_records_hint: &'static str,
    success_hint: &'static str,
    retry_action: &'static str,
}

fn run_report_lines(
    repo_name: &str,
    report: &GitMirrorRunReport,
    copy: RunReportCopy,
) -> Vec<String> {
    let mut lines = vec![format!(
        "{}[{repo_name}]: attempted={} committed={} out_of_sync={} skipped={}",
        copy.header, report.attempted, report.committed, report.out_of_sync, report.skipped
    )];
    let retry_command = git_command(copy.retry_action, repo_name, true);
    for (index, record) in report.records.iter().enumerate() {
        lines.extend(record_detail_lines(
            "record",
            index + 1,
            record,
            None,
            Some(retry_command.as_str()),
        ));
    }
    if report.attempted == 0 {
        lines.push(format!("  {}: {}", copy.hint_label, copy.no_records_hint));
    } else if report.out_of_sync > 0 {
        lines.push(format!(
            "  repair_hint: fix the reported failure_location/error; retry with `{}`",
            retry_command
        ));
    } else {
        lines.push(format!("  {}: {}", copy.hint_label, copy.success_hint));
    }
    lines
}

fn record_detail_lines(
    prefix: &str,
    index: usize,
    record: &GitMirrorRecord,
    now_ms: Option<i64>,
    retry_command: Option<&str>,
) -> Vec<String> {
    let mut detail = format!(
        "  {prefix}[{index}]: deve_commit={} state={} ledger_seq={} attempts={} git_commit={}",
        record.deve_commit_id,
        record.state.as_str(),
        record.ledger_seq,
        record.attempts,
        record.git_commit_id.as_deref().unwrap_or("-")
    );
    if let Some(now_ms) = now_ms {
        detail.push_str(&format!(
            " queued_lag_ms={} updated_lag_ms={}",
            lag_ms(now_ms, record.queued_at_ms),
            lag_ms(now_ms, record.updated_at_ms)
        ));
    }
    detail.push_str(&format!(
        " queued_at_ms={} updated_at_ms={}",
        record.queued_at_ms, record.updated_at_ms
    ));

    let mut lines = vec![detail];
    if let Some(error) = &record.last_error {
        let location = record
            .failure_stage
            .map(|stage| stage.as_str())
            .unwrap_or_else(|| failure_location(error));
        lines.push(format!(
            "  failure[{index}]: location={} error={}",
            location, error
        ));
        if record.failure_subject.is_some()
            || record.failure_command.is_some()
            || record.failure_exit_status.is_some()
        {
            lines.push(format!(
                "  failure_meta[{index}]: subject={} command={} exit_status={}",
                record.failure_subject.as_deref().unwrap_or("-"),
                record.failure_command.as_deref().unwrap_or("-"),
                record.failure_exit_status.as_deref().unwrap_or("-")
            ));
        }
        if let Some(action) = GitMirrorRepairAction::for_record(record) {
            lines.push(format!(
                "  repair_action[{index}]: code={} retryable_after_fix={} subject={}",
                action.code.as_str(),
                yes_no(action.retryable_after_fix),
                action.subject.as_deref().unwrap_or("-")
            ));
            lines.push(format!(
                "  repair_guidance[{index}]: manual_only=yes next={} retry_command={}",
                repair_next_step(action.code),
                repair_retry_command(&action, retry_command)
            ));
        }
    }
    lines
}

fn repair_next_step(code: GitMirrorRepairActionCode) -> &'static str {
    code.next_step()
}

fn repair_retry_command(action: &GitMirrorRepairAction, retry_command: Option<&str>) -> String {
    if !action.retryable_after_fix {
        return "-".to_string();
    }
    retry_command
        .map(|command| format!("`{command}`"))
        .unwrap_or_else(|| "-".to_string())
}

fn lag_ms(now_ms: i64, timestamp_ms: i64) -> i64 {
    now_ms.saturating_sub(timestamp_ms).max(0)
}

fn failure_location(error: &str) -> &'static str {
    let normalized = error.to_ascii_lowercase();
    if normalized.contains("pending source-control")
        || normalized.contains("staged source-control")
        || normalized.contains("pending_fs")
        || normalized.contains("staging")
    {
        return "deve_source_control";
    }
    if normalized.contains(".notegit") || normalized.contains("tracked by git") {
        return "notegit_protection";
    }
    if normalized.contains("outside queued deve commit")
        || normalized.contains("unsafe projection path")
        || normalized.contains("projection diff")
    {
        return "projection_scope";
    }
    if normalized.contains("parent")
        || normalized.contains("git head does not match")
        || normalized.contains("not mirrored")
    {
        return "git_history_mapping";
    }
    if normalized.contains("worktree") || normalized.contains("rev-parse") {
        return "git_worktree";
    }
    if normalized.contains("git ") {
        return "git_command";
    }
    "mirror_executor"
}

fn mirror_command(repo_name: &str, retry_out_of_sync: bool) -> String {
    let retry = if retry_out_of_sync {
        " --retry-out-of-sync"
    } else {
        ""
    };
    format!(
        "deve_cli git mirror --repo {}{}",
        shell_quote(repo_name),
        retry
    )
}

fn git_command(action: &str, repo_name: &str, retry_out_of_sync: bool) -> String {
    let retry = if retry_out_of_sync {
        " --retry-out-of-sync"
    } else {
        ""
    };
    format!(
        "deve_cli git {} --repo {}{}",
        action,
        shell_quote(repo_name),
        retry
    )
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':'))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn bool_label(value: bool) -> &'static str {
    if value { "present" } else { "missing" }
}

fn protection_label(value: bool) -> &'static str {
    if value { "protected" } else { "missing" }
}

fn queue_state_label(summary: &GitMirrorSummary) -> &'static str {
    if summary.out_of_sync > 0 {
        "out_of_sync"
    } else if summary.queued > 0 {
        "queued"
    } else {
        "clean"
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
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
#[path = "git_output_test.rs"]
mod tests;

#[cfg(test)]
#[path = "git_output_repair_action_test.rs"]
mod repair_action_tests;

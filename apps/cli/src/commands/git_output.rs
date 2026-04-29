//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!   - 12_commands#cli-commands
//!
//! Human-readable Git mirror diagnostics.

use deve_core::git_bridge::{
    GitMirrorCommitState, GitMirrorRecord, GitMirrorRunReport, GitMirrorStatus, GitMirrorSummary,
};

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
    for (index, record) in lagging.into_iter().enumerate() {
        lines.extend(record_detail_lines("lag", index + 1, record, Some(now_ms)));
    }

    if summary.queued > 0 {
        lines.push(format!(
            "  next_action: run `{}` to mirror queued Deve commit(s)",
            mirror_command(repo_name, false)
        ));
    }
    if summary.out_of_sync > 0 {
        lines.push(format!(
            "  repair_hint: fix the reported failure_location/error, then run `{}`",
            mirror_command(repo_name, true)
        ));
    }
    lines
}

fn mirror_report_lines(repo_name: &str, report: &GitMirrorRunReport) -> Vec<String> {
    let mut lines = vec![format!(
        "git_mirror[{repo_name}]: attempted={} committed={} out_of_sync={} skipped={}",
        report.attempted, report.committed, report.out_of_sync, report.skipped
    )];
    for (index, record) in report.records.iter().enumerate() {
        lines.extend(record_detail_lines("record", index + 1, record, None));
    }
    if report.attempted == 0 {
        lines.push("  mirror_hint: no queued Git mirror records".to_string());
    } else if report.out_of_sync > 0 {
        lines.push(format!(
            "  repair_hint: fix the reported failure_location/error; retry with `{}`",
            mirror_command(repo_name, true)
        ));
    } else {
        lines.push("  mirror_hint: attempted records are mirrored".to_string());
    }
    lines
}

fn record_detail_lines(
    prefix: &str,
    index: usize,
    record: &GitMirrorRecord,
    now_ms: Option<i64>,
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
    }
    lines
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

#[cfg(test)]
#[path = "git_output_test.rs"]
mod tests;

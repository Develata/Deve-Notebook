//! plan_ref:
//!   - 07_diff_logic#git-mirror-lifecycle
//!   - 12_commands#cli-commands
//!
//! Per-record Git mirror diagnostics rendering.

use deve_core::git_bridge::{GitMirrorRecord, GitMirrorRepairAction, GitMirrorRepairActionCode};

pub(super) fn record_detail_lines(
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

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

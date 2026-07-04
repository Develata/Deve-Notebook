//! plan_ref:
//!   - 03_storage/index#git-ecosystem-coexistence
//!   - 05_diff_logic#git-mirror-lifecycle
//!   - 14_commands#cli-commands
//!
//! Git mirror status summary rendering.

use deve_core::git_bridge::{
    GitMirrorCommitState, GitMirrorRecord, GitMirrorStatus, GitMirrorSummary,
};

use super::command::{mirror_command, ngit_command};
use super::record::record_detail_lines;

pub(super) fn status_lines(
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

pub(super) fn status_lines_at(
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

    let retry_command = ngit_command("export", repo_name, true);
    let mut lagging_count = 0usize;
    let mut lagging_lines = Vec::new();
    for record in records.iter().filter(|record| {
        matches!(
            record.state,
            GitMirrorCommitState::Queued | GitMirrorCommitState::OutOfSync
        )
    }) {
        lagging_count += 1;
        lagging_lines.extend(record_detail_lines(
            "lag",
            lagging_count,
            record,
            Some(now_ms),
            Some(retry_command.as_str()),
        ));
    }

    if lagging_count == 0 {
        lines.push("  lagging_records=0".to_string());
        return lines;
    }

    lines.push(format!("  lagging_records={lagging_count}"));
    lines.extend(lagging_lines);

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

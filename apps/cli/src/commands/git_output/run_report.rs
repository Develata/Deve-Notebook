//! plan_ref:
//!   - 05_diff_logic#git-mirror-lifecycle
//!   - 14_commands#cli-commands
//!
//! Git mirror/export execution report rendering.

use deve_core::git_bridge::GitMirrorRunReport;

use super::command::git_command;
use super::record::record_detail_lines;

pub(super) fn mirror_report_lines(repo_name: &str, report: &GitMirrorRunReport) -> Vec<String> {
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

pub(super) fn export_report_lines(repo_name: &str, report: &GitMirrorRunReport) -> Vec<String> {
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

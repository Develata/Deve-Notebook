//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!   - 12_commands#cli-commands
//!
//! Human-readable Git mirror diagnostics.

use deve_core::git_bridge::{
    GitImportApplyReport, GitImportPlan, GitMirrorRecord, GitMirrorRunReport, GitMirrorStatus,
    GitMirrorSummary,
};

#[path = "git_output_command.rs"]
mod command;
#[path = "git_import_output.rs"]
mod import_output;
#[path = "git_push_output.rs"]
mod push_output;
#[path = "git_output_record.rs"]
mod record;
#[path = "git_output_run_report.rs"]
mod run_report;
#[path = "git_output_status.rs"]
mod status;

use import_output::{import_apply_report_lines, import_plan_lines};
pub(super) use push_output::print_push_report;
use run_report::{export_report_lines, mirror_report_lines};
use status::status_lines;
#[cfg(test)]
use status::status_lines_at;

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

#[cfg(test)]
#[path = "git_output_test.rs"]
mod tests;

#[cfg(test)]
#[path = "git_output_repair_action_test.rs"]
mod repair_action_tests;

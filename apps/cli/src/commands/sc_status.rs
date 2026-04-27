//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 12_commands#cli-commands
//!
//! Read-only Source Control status diagnostics for local smoke-test hygiene.

use crate::commands::repo_arg::resolve_local_repo_args;
use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::source_control::ChangeEntry;
use std::path::Path;

pub fn run(ledger_dir: &Path, target_repo: Option<&str>, snapshot_depth: usize) -> Result<()> {
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo_names = resolve_local_repo_args(&repo, target_repo)?;
    for repo_name in repo_names {
        let status = collect_repo_status(&repo, &repo_name)?;
        print_repo_status(&status);
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct ScStatusReport {
    repo_name: String,
    staged: Vec<ChangeEntry>,
    unstaged: Vec<ChangeEntry>,
}

fn collect_repo_status(repo: &RepoManager, repo_name: &str) -> Result<ScStatusReport> {
    Ok(ScStatusReport {
        repo_name: repo_name.to_string(),
        staged: repo.list_staged_in_local_repo(repo_name)?,
        unstaged: repo.list_pending_fs_in_local_repo(repo_name)?,
    })
}

fn print_repo_status(report: &ScStatusReport) {
    println!(
        "sc_status[{}]: staged={} unstaged={}",
        report.repo_name,
        report.staged.len(),
        report.unstaged.len()
    );
    print_entries("staged", &report.staged);
    print_entries("unstaged", &report.unstaged);
}

fn print_entries(label: &str, entries: &[ChangeEntry]) {
    if entries.is_empty() {
        return;
    }
    println!("{label}:");
    for entry in entries {
        println!("  {:?} {}", entry.status, entry.path);
    }
}

#[cfg(test)]
mod tests {
    use super::{ScStatusReport, print_repo_status};
    use deve_core::source_control::{ChangeEntry, ChangeStatus};

    #[test]
    fn print_status_handles_clean_repo() {
        print_repo_status(&ScStatusReport {
            repo_name: "default".into(),
            staged: vec![],
            unstaged: vec![],
        });
    }

    #[test]
    fn status_report_keeps_staged_and_unstaged_separate() {
        let report = ScStatusReport {
            repo_name: "default".into(),
            staged: vec![entry("a.md", ChangeStatus::Added)],
            unstaged: vec![entry("b.md", ChangeStatus::Modified)],
        };

        assert_eq!(report.staged.len(), 1);
        assert_eq!(report.unstaged.len(), 1);
    }

    fn entry(path: &str, status: ChangeStatus) -> ChangeEntry {
        ChangeEntry {
            path: path.into(),
            renamed_from: None,
            doc_id: None,
            status,
            has_conflict: false,
        }
    }
}

//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 14_commands#cli-commands
//!
//! Read-only Source Control status diagnostics for local smoke-test hygiene.

use crate::admin_api::ScStatusResponse;
use crate::commands::live_proxy;
use crate::commands::repo_arg::resolve_local_repo_args;
use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::source_control::ChangeEntry;
use std::path::Path;

pub fn run(ledger_dir: &Path, target_repo: Option<&str>, snapshot_depth: usize) -> Result<()> {
    let repo = match RepoManager::init(ledger_dir, snapshot_depth, None, None) {
        Ok(repo) => repo,
        Err(err) if live_proxy::is_db_lock_error(&err) => {
            for report in live_proxy::sc_status(ledger_dir, target_repo)? {
                print_repo_status(&report);
            }
            return Ok(());
        }
        Err(err) => return Err(err),
    };
    let repo_names = resolve_local_repo_args(&repo, target_repo)?;
    for repo_name in repo_names {
        let status = collect_repo_status(&repo, &repo_name)?;
        print_repo_status(&status);
    }
    Ok(())
}

fn collect_repo_status(repo: &RepoManager, repo_name: &str) -> Result<ScStatusResponse> {
    Ok(ScStatusResponse {
        repo_name: repo_name.to_string(),
        staged: repo.list_staged_in_local_repo(repo_name)?,
        unstaged: repo.list_pending_fs_in_local_repo(repo_name)?,
    })
}

pub(crate) fn print_repo_status(report: &ScStatusResponse) {
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
    use super::print_repo_status;
    use crate::admin_api::ScStatusResponse;
    use deve_core::source_control::{ChangeEntry, ChangeStatus};

    #[test]
    fn print_status_handles_clean_repo() {
        print_repo_status(&ScStatusResponse {
            repo_name: "default".into(),
            staged: vec![],
            unstaged: vec![],
        });
    }

    #[test]
    fn status_report_keeps_staged_and_unstaged_separate() {
        let report = ScStatusResponse {
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

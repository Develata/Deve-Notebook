//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!   - 12_commands#cli-commands
//!
//! Read-only Git mirror bridge diagnostics.

use crate::commands::repo_arg::resolve_local_repo_args;
use anyhow::Result;
use deve_core::git_bridge::{GitMirrorStatus, GitMirrorSummary};
use deve_core::ledger::RepoManager;
use std::path::Path;

pub fn status(
    ledger_dir: &Path,
    vault_root: &Path,
    target_repo: Option<&str>,
    snapshot_depth: usize,
) -> Result<()> {
    let mut repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    repo.set_vault_root(vault_root);
    let repo_names = resolve_local_repo_args(&repo, target_repo)?;
    for repo_name in repo_names {
        let repo_root = repo.local_repo_workspace_root(&repo_name)?;
        let status = deve_core::git_bridge::inspect_repo_root(&repo_root)?;
        let summary =
            repo.run_on_local_repo(&repo_name, deve_core::git_bridge::summarize_records)?;
        print_status(&repo_name, &status, &summary);
    }
    Ok(())
}

fn print_status(repo_name: &str, status: &GitMirrorStatus, summary: &GitMirrorSummary) {
    println!(
        "git_status[{repo_name}]: state={} queue_state={} git={} notegit={} gitignore_notegit={} queued={} committed={} out_of_sync={}",
        status.state.as_str(),
        queue_state_label(summary),
        status.git_metadata_kind.as_str(),
        bool_label(status.notegit_present),
        protection_label(status.gitignore_protects_notegit),
        summary.queued,
        summary.committed,
        summary.out_of_sync
    );
    if let Some(reason) = &status.reason {
        println!("  reason: {reason}");
    }
    println!("  repo_root: {}", status.repo_root.display());
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
mod tests {
    use super::print_status;
    use deve_core::git_bridge::{
        GitMetadataKind, GitMirrorState, GitMirrorStatus, GitMirrorSummary,
    };

    #[test]
    fn print_git_status_handles_disabled_repo() {
        print_status(
            "default",
            &GitMirrorStatus {
                repo_root: std::path::PathBuf::from("vault/default"),
                notegit_present: true,
                git_metadata_kind: GitMetadataKind::Missing,
                gitignore_protects_notegit: true,
                state: GitMirrorState::Disabled,
                reason: None,
            },
            &GitMirrorSummary::default(),
        );
    }

    #[test]
    fn print_git_status_handles_out_of_sync_summary() {
        print_status(
            "default",
            &GitMirrorStatus {
                repo_root: std::path::PathBuf::from("vault/default"),
                notegit_present: true,
                git_metadata_kind: GitMetadataKind::Directory,
                gitignore_protects_notegit: true,
                state: GitMirrorState::Ready,
                reason: None,
            },
            &GitMirrorSummary {
                queued: 1,
                committed: 2,
                out_of_sync: 1,
            },
        );
    }
}

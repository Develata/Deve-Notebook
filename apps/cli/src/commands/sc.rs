//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 14_commands#cli-commands
//!
//! Minimal Deve Source Control CLI surface.

use crate::commands::repo_arg::resolve_local_repo_args;
use anyhow::{Result, bail};
use clap::Subcommand;
use deve_core::config::GitBridgeMode;
use deve_core::ledger::RepoManager;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeEntry;
use std::path::Path;

#[derive(Subcommand, Debug)]
pub(crate) enum ScAction {
    /// Print staged and unstaged source-control counts
    Status {
        #[arg(long)]
        repo: Option<String>,
    },
    /// Stage pending source-control changes
    Stage {
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        all: bool,
    },
    /// Commit staged source-control changes
    Commit {
        #[arg(long)]
        repo: Option<String>,
        #[arg(short, long)]
        message: String,
    },
}

pub fn status(ledger_dir: &Path, target_repo: Option<&str>, snapshot_depth: usize) -> Result<()> {
    crate::commands::sc_status::run(ledger_dir, target_repo, snapshot_depth)
}

pub fn stage(
    ledger_dir: &Path,
    target_repo: Option<&str>,
    all: bool,
    snapshot_depth: usize,
) -> Result<()> {
    require_stage_all(all)?;
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo_names = resolve_local_repo_args(&repo, target_repo)?;
    for repo_name in repo_names {
        let pending = repo.list_pending_fs_in_local_repo(&repo_name)?;
        let targets = targets_from_entries(&pending);
        repo.stage_resolved_pending_targets_in_local_repo(&repo_name, &targets)?;
        println!("sc_stage[{repo_name}]: staged={}", targets.len());
    }
    Ok(())
}

pub fn commit(
    ledger_dir: &Path,
    target_repo: Option<&str>,
    message: &str,
    snapshot_depth: usize,
    git_bridge: GitBridgeMode,
) -> Result<()> {
    let message = message.trim();
    if message.is_empty() {
        bail!("sc commit requires a non-empty --message");
    }
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo_names = resolve_local_repo_args(&repo, target_repo)?;
    for repo_name in repo_names {
        let commit =
            repo.commit_staged_in_local_repo_with_git_bridge(&repo_name, message, git_bridge)?;
        println!(
            "sc_commit[{repo_name}]: id={} ledger_seq={} files={}",
            commit.id, commit.ledger_seq, commit.doc_count
        );
    }
    Ok(())
}

fn require_stage_all(all: bool) -> Result<()> {
    if !all {
        bail!("sc stage currently requires --all");
    }
    Ok(())
}

fn targets_from_entries(entries: &[ChangeEntry]) -> Vec<ScPathTarget> {
    entries
        .iter()
        .map(|entry| ScPathTarget {
            path: entry.path.clone(),
            doc_id: entry.doc_id,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{require_stage_all, targets_from_entries};
    use deve_core::models::DocId;
    use deve_core::source_control::{ChangeEntry, ChangeStatus};

    #[test]
    fn stage_requires_explicit_all() {
        assert!(require_stage_all(false).is_err());
        assert!(require_stage_all(true).is_ok());
    }

    #[test]
    fn targets_preserve_doc_identity() {
        let doc_id = DocId::from_u128(7);
        let targets = targets_from_entries(&[ChangeEntry {
            path: "gone.md".into(),
            renamed_from: None,
            doc_id: Some(doc_id),
            status: ChangeStatus::Deleted,
            has_conflict: false,
        }]);

        assert_eq!(targets[0].path, "gone.md");
        assert_eq!(targets[0].doc_id, Some(doc_id));
    }
}

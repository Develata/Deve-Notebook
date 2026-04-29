//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!   - 12_commands#cli-commands
//!
//! Git mirror bridge diagnostics and explicit executor commands.

use super::git_output::{print_export_report, print_mirror_report, print_status};
use crate::commands::repo_arg::resolve_local_repo_args;
use anyhow::Result;
use deve_core::git_bridge::GitMirrorRunOptions;
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
        let records = repo.run_on_local_repo(&repo_name, deve_core::git_bridge::list_records)?;
        print_status(&repo_name, &status, &summary, &records);
    }
    Ok(())
}

pub fn mirror(
    ledger_dir: &Path,
    vault_root: &Path,
    target_repo: Option<&str>,
    retry_out_of_sync: bool,
    snapshot_depth: usize,
) -> Result<()> {
    run_executor(
        ledger_dir,
        vault_root,
        target_repo,
        retry_out_of_sync,
        snapshot_depth,
        print_mirror_report,
    )
}

pub fn export(
    ledger_dir: &Path,
    vault_root: &Path,
    target_repo: Option<&str>,
    retry_out_of_sync: bool,
    snapshot_depth: usize,
) -> Result<()> {
    run_executor(
        ledger_dir,
        vault_root,
        target_repo,
        retry_out_of_sync,
        snapshot_depth,
        print_export_report,
    )
}

fn run_executor(
    ledger_dir: &Path,
    vault_root: &Path,
    target_repo: Option<&str>,
    retry_out_of_sync: bool,
    snapshot_depth: usize,
    print_report: fn(&str, &deve_core::git_bridge::GitMirrorRunReport),
) -> Result<()> {
    let mut repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    repo.set_vault_root(vault_root);
    let repo_names = resolve_local_repo_args(&repo, target_repo)?;
    for repo_name in repo_names {
        let repo_root = repo.local_repo_workspace_root(&repo_name)?;
        let report = repo.run_on_local_repo(&repo_name, |db| {
            deve_core::git_bridge::run_pending_mirror(
                db,
                &repo_root,
                GitMirrorRunOptions { retry_out_of_sync },
            )
        })?;
        print_report(&repo_name, &report);
    }
    Ok(())
}

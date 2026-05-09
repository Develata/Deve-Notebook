//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!   - 12_commands#cli-commands
//!
//! Git mirror bridge diagnostics and explicit executor commands.

use super::git_output::{
    print_export_report, print_import_apply_report, print_import_plan, print_mirror_report,
    print_push_report, print_status,
};
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
        let summary = repo.run_on_local_repo(&repo_name, |db| {
            Ok(deve_core::git_bridge::summarize_records(db)?)
        })?;
        let records = repo.run_on_local_repo(&repo_name, |db| {
            Ok(deve_core::git_bridge::list_records(db)?)
        })?;
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
        run_mirror_for_repo,
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
        run_export_for_repo,
        print_export_report,
    )
}

pub fn import(
    ledger_dir: &Path,
    vault_root: &Path,
    target_repo: Option<&str>,
    apply: bool,
    snapshot_depth: usize,
) -> Result<()> {
    let mut repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    repo.set_vault_root(vault_root);
    let repo_names = resolve_local_repo_args(&repo, target_repo)?;
    for repo_name in repo_names {
        let repo_root = repo.local_repo_workspace_root(&repo_name)?;
        if apply {
            let report = deve_core::git_bridge::apply_import(&repo, &repo_name, &repo_root)?;
            print_import_apply_report(&repo_name, &report);
        } else {
            let plan = deve_core::git_bridge::plan_import(&repo_root)?;
            print_import_plan(&repo_name, &plan);
        }
    }
    Ok(())
}

pub fn push(
    ledger_dir: &Path,
    vault_root: &Path,
    target_repo: Option<&str>,
    remote: Option<&str>,
    branch: Option<&str>,
    snapshot_depth: usize,
) -> Result<()> {
    let mut repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    repo.set_vault_root(vault_root);
    let repo_names = resolve_local_repo_args(&repo, target_repo)?;
    for repo_name in repo_names {
        let repo_root = repo.local_repo_workspace_root(&repo_name)?;
        let report = repo.run_on_local_repo(&repo_name, |db| {
            deve_core::git_bridge::push_mirror(
                db,
                &repo_root,
                deve_core::git_bridge::GitMirrorPushOptions {
                    remote: remote.map(str::to_string),
                    branch: branch.map(str::to_string),
                },
            )
        })?;
        print_push_report(&repo_name, &report);
    }
    Ok(())
}

fn run_executor(
    ledger_dir: &Path,
    vault_root: &Path,
    target_repo: Option<&str>,
    retry_out_of_sync: bool,
    snapshot_depth: usize,
    run_report: fn(
        &RepoManager,
        &str,
        &Path,
        bool,
    ) -> Result<deve_core::git_bridge::GitMirrorRunReport>,
    print_report: fn(&str, &deve_core::git_bridge::GitMirrorRunReport),
) -> Result<()> {
    let mut repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    repo.set_vault_root(vault_root);
    let repo_names = resolve_local_repo_args(&repo, target_repo)?;
    for repo_name in repo_names {
        let repo_root = repo.local_repo_workspace_root(&repo_name)?;
        let report = run_report(&repo, &repo_name, &repo_root, retry_out_of_sync)?;
        print_report(&repo_name, &report);
    }
    Ok(())
}

fn run_mirror_for_repo(
    repo: &RepoManager,
    repo_name: &str,
    repo_root: &Path,
    retry_out_of_sync: bool,
) -> Result<deve_core::git_bridge::GitMirrorRunReport> {
    repo.run_on_local_repo(repo_name, |db| {
        deve_core::git_bridge::run_pending_mirror(
            db,
            repo_root,
            GitMirrorRunOptions { retry_out_of_sync },
        )
    })
}

fn run_export_for_repo(
    repo: &RepoManager,
    repo_name: &str,
    repo_root: &Path,
    retry_out_of_sync: bool,
) -> Result<deve_core::git_bridge::GitMirrorRunReport> {
    let repo_info = repo
        .get_repo_info_for(None, Some(repo_name))?
        .ok_or_else(|| anyhow::anyhow!("Local repo metadata is missing for {repo_name}"))?;
    repo.run_on_local_repo(repo_name, |db| {
        deve_core::git_bridge::export_mirror(
            db,
            repo_root,
            repo_info.uuid,
            GitMirrorRunOptions { retry_out_of_sync },
        )
    })
}

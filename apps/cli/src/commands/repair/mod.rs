//! plan_ref:
//!   - 14_commands#cli-commands
//!   - 04_repository#repo-catalog-repair-contract
//!   - 04_repository#tree-projection-contract

#[cfg(test)]
mod catalog_test;
#[cfg(test)]
mod check_test;
mod path_fix;
mod rebuild;
mod restore;
mod shadow;
mod weird_paths;

use crate::commands::repo_arg::resolve_local_repo_args;
use anyhow::{Result, bail};
use deve_core::ledger::RepoManager;
use deve_core::sync::{ProjectionDiagnosticStatus, SyncManager};
use std::path::Path;
use std::sync::Arc;

pub struct RepairOptions<'a> {
    pub backup_root: &'a Path,
    pub target_repo: Option<&'a str>,
    pub paths: &'a [String],
    pub rebuild_projection: bool,
    pub check: bool,
}

pub fn run(ledger_dir: &Path, snapshot_depth: usize, options: RepairOptions<'_>) -> Result<()> {
    let quarantined = if options.check {
        0
    } else {
        shadow::quarantine_nil_shadow_repos(&ledger_dir.join("remotes"))?
    };
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    if !options.check {
        repo.repair_local_repo_catalog()?;
    }
    let repo = Arc::new(repo);

    let repo_names = resolve_local_repo_args(&repo, options.target_repo)?;
    if options.check {
        return check_repair_readiness(repo, &repo_names);
    }

    let sync_manager = SyncManager::new_checked(repo.clone())?;
    let fixed_paths = path_fix::repair_repo_prefixed_paths(&repo, &repo_names)?;
    let quarantined_md_dirs = weird_paths::quarantine_md_dirs(&repo, &repo_names)?;
    let restored = restore::restore_docs_from_backup(
        &repo,
        &sync_manager,
        options.backup_root,
        &repo_names,
        options.paths,
    )?;
    let rebuild_report = if options.rebuild_projection {
        rebuild::rebuild_repos(&sync_manager, &repo_names)?
    } else {
        rebuild::RebuildReport::default()
    };

    println!(
        "repair: quarantined_nil_shadows={} fixed_repo_paths={} quarantined_md_dirs={} restored_docs={} rebuilt_repos={} authority_corrupt_repos={}",
        quarantined,
        fixed_paths,
        quarantined_md_dirs,
        restored,
        rebuild_report.rebuilt,
        rebuild_report.authority_corrupt.len()
    );
    if !rebuild_report.authority_corrupt.is_empty() {
        let corrupt_summary = rebuild_report
            .authority_corrupt
            .iter()
            .map(|item| format!("{}:{}", item.repo_name, item.code))
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "repair: {} repo(s) have corrupted Structure Facts authority; projection rebuild skipped: {}",
            rebuild_report.authority_corrupt.len(),
            corrupt_summary
        );
    }
    Ok(())
}

fn check_repair_readiness(repo: Arc<RepoManager>, repo_names: &[String]) -> Result<()> {
    let sync_manager = SyncManager::new_checked(repo)?;
    let mut authority_corrupt = Vec::new();
    for repo_name in repo_names {
        let diagnostic = sync_manager.diagnose_projection_local_repo(repo_name)?;
        println!(
            "repair-check[{}]: status={} rebuild_supported={}",
            repo_name,
            projection_status(&diagnostic.status),
            diagnostic.rebuild_supported
        );
        println!("repair_hint: {}", diagnostic.repair_hint);
        if let Some(issue) = diagnostic.issue.as_ref() {
            println!("issue_code: {}", issue.code);
            println!("issue_detail: {}", issue.detail);
            if diagnostic.status == ProjectionDiagnosticStatus::AuthorityCorrupt {
                authority_corrupt.push(format!("{}:{}", repo_name, issue.code));
            }
        } else if diagnostic.status == ProjectionDiagnosticStatus::AuthorityCorrupt {
            authority_corrupt.push(repo_name.clone());
        }
    }
    println!(
        "repair-check: repos={} authority_corrupt_repos={}",
        repo_names.len(),
        authority_corrupt.len()
    );
    if !authority_corrupt.is_empty() {
        bail!(
            "repair-check: {} repo(s) have corrupted Structure Facts authority; repair steps must remain disabled until authority is restored: {}",
            authority_corrupt.len(),
            authority_corrupt.join(", ")
        );
    }
    Ok(())
}

fn projection_status(status: &ProjectionDiagnosticStatus) -> &'static str {
    match status {
        ProjectionDiagnosticStatus::Healthy => "healthy",
        ProjectionDiagnosticStatus::AuthorityCorrupt => "authority_corrupt",
    }
}

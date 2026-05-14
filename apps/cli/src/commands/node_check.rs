//! plan_ref:
//!   - 12_commands#cli-commands
//!   - 06_repository#tree-projection-contract

use crate::admin_api::{NodeCheckResponse, ProjectionCheckResponse};
use crate::commands::live_proxy;
use crate::commands::repo_arg::resolve_local_repo_args;
use anyhow::{Result, bail};
use deve_core::ledger::RepoManager;
use deve_core::ledger::node_check::{check_node_consistency, repair_missing_nodes};
use deve_core::sync::SyncManager;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn run(
    ledger_dir: &PathBuf,
    vault_path: &Path,
    snapshot_depth: usize,
    repair: bool,
    projection: bool,
    repo_name: Option<String>,
) -> Result<()> {
    let repo = match RepoManager::init(ledger_dir, snapshot_depth, None, None) {
        Ok(repo) => repo,
        Err(err) if live_proxy::is_db_lock_error(&err) => {
            if projection {
                let reports = live_proxy::projection_check(ledger_dir, repo_name.as_deref())?;
                return print_projection_reports(&reports);
            }
            let reports = live_proxy::node_check(ledger_dir, repo_name.as_deref(), repair)?;
            return print_reports(&reports);
        }
        Err(err) => return Err(err),
    };
    if projection {
        let reports = collect_projection_reports(Arc::new(repo), vault_path, repo_name.as_deref())?;
        return print_projection_reports(&reports);
    }
    let reports = collect_reports(&repo, repo_name.as_deref(), repair)?;
    print_reports(&reports)
}

fn collect_reports(
    repo: &RepoManager,
    target_repo: Option<&str>,
    repair: bool,
) -> Result<Vec<NodeCheckResponse>> {
    let repo_names = resolve_local_repo_args(repo, target_repo)?;
    let mut reports = Vec::with_capacity(repo_names.len());
    for repo_name in repo_names {
        let report = repo.run_on_local_repo(&repo_name, |db| {
            if repair {
                repair_missing_nodes(db)
            } else {
                check_node_consistency(db)
            }
        })?;
        reports.push(NodeCheckResponse {
            repo_name,
            missing_nodes: report.missing_nodes,
            orphan_nodes: report.orphan_nodes,
        });
    }
    Ok(reports)
}

fn collect_projection_reports(
    repo: Arc<RepoManager>,
    vault_path: &Path,
    target_repo: Option<&str>,
) -> Result<Vec<ProjectionCheckResponse>> {
    let repo_names = resolve_local_repo_args(&repo, target_repo)?;
    let sync_manager = SyncManager::new_checked(repo, vault_path.to_path_buf())?;
    let mut reports = Vec::with_capacity(repo_names.len());
    for repo_name in repo_names {
        let diagnostic = sync_manager.diagnose_projection_local_repo(&repo_name)?;
        reports.push(ProjectionCheckResponse::from_diagnostic(diagnostic));
    }
    Ok(reports)
}

fn print_reports(reports: &[NodeCheckResponse]) -> Result<()> {
    for report in reports {
        println!(
            "node_check[{}]: missing_nodes={} orphan_nodes={}",
            report.repo_name,
            report.missing_nodes.len(),
            report.orphan_nodes.len()
        );
        if !report.missing_nodes.is_empty() {
            println!("missing_nodes:");
            for (doc_id, path) in &report.missing_nodes {
                println!("  {} {}", doc_id, path);
            }
        }
        if !report.orphan_nodes.is_empty() {
            println!("orphan_nodes:");
            for (node_id, path) in &report.orphan_nodes {
                println!("  {} {}", node_id, path);
            }
        }
    }
    Ok(())
}

fn print_projection_reports(reports: &[ProjectionCheckResponse]) -> Result<()> {
    let mut authority_corrupt = Vec::new();
    for report in reports {
        println!(
            "projection_check[{}]: status={} rebuild_supported={}",
            report.repo_name, report.status, report.rebuild_supported
        );
        println!("repair_hint: {}", report.repair_hint);
        if let Some(code) = &report.issue_code {
            println!("issue_code: {}", code);
        }
        if let Some(detail) = &report.issue_detail {
            println!("issue_detail: {}", detail);
        }
        if report.status == "authority_corrupt" {
            authority_corrupt.push(authority_corrupt_summary(report));
        }
    }
    if !authority_corrupt.is_empty() {
        bail!(
            "projection check failed closed: Structure Facts authority corrupt in {} repo(s): {}",
            authority_corrupt.len(),
            authority_corrupt.join(", ")
        );
    }
    Ok(())
}

fn authority_corrupt_summary(report: &ProjectionCheckResponse) -> String {
    match report.issue_code.as_deref() {
        Some(code) => format!("{}:{}", report.repo_name, code),
        None => report.repo_name.clone(),
    }
}

#[cfg(test)]
mod tests;

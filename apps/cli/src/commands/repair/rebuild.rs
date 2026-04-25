//! plan_ref:
//!   - 12_commands#cli-commands
//!   - 06_repository#tree-projection-contract

use anyhow::Result;
use deve_core::sync::{ProjectionDiagnosticStatus, SyncManager};

#[derive(Debug, Default)]
pub(super) struct RebuildReport {
    pub rebuilt: usize,
    pub authority_corrupt: usize,
}

pub(super) fn rebuild_repos(
    sync_manager: &SyncManager,
    repo_names: &[String],
) -> Result<RebuildReport> {
    let mut report = RebuildReport::default();
    for repo_name in repo_names {
        let diagnostic = sync_manager.diagnose_projection_local_repo(repo_name)?;
        if diagnostic.status == ProjectionDiagnosticStatus::AuthorityCorrupt {
            print_authority_corruption(repo_name, &diagnostic);
            report.authority_corrupt += 1;
            continue;
        }
        sync_manager.rebuild_projection_local_repo(repo_name)?;
        println!("repair: rebuilt projection for repo {}", repo_name);
        report.rebuilt += 1;
    }
    Ok(report)
}

fn print_authority_corruption(repo_name: &str, diagnostic: &deve_core::sync::ProjectionDiagnostic) {
    let (code, detail) = diagnostic
        .issue
        .as_ref()
        .map(|issue| (issue.code.as_str(), issue.detail.as_str()))
        .unwrap_or(("structure_authority", "unknown Structure Facts corruption"));
    println!(
        "repair: projection authority corrupt repo={} code={} detail={}",
        repo_name, code, detail
    );
    println!(
        "repair: skip projection rebuild for repo {}; rebuild cannot modify Structure Facts authority",
        repo_name
    );
}

#[cfg(test)]
#[path = "rebuild_test.rs"]
mod tests;

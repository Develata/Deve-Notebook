//! plan_ref:
//!   - 12_commands#cli-commands
//!   - 06_repository#tree-projection-contract

use anyhow::Result;
use deve_core::sync::{ProjectionDiagnosticStatus, SyncManager};

#[derive(Debug, Default)]
pub(super) struct RebuildReport {
    pub rebuilt: usize,
    pub authority_corrupt: Vec<AuthorityCorruption>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AuthorityCorruption {
    pub repo_name: String,
    pub code: String,
    pub detail: String,
}

pub(super) fn rebuild_repos(
    sync_manager: &SyncManager,
    repo_names: &[String],
) -> Result<RebuildReport> {
    let mut report = RebuildReport::default();
    for repo_name in repo_names {
        let diagnostic = sync_manager.diagnose_projection_local_repo(repo_name)?;
        if diagnostic.status == ProjectionDiagnosticStatus::AuthorityCorrupt {
            let corruption = authority_corruption(repo_name, &diagnostic);
            print_authority_corruption(&corruption);
            report.authority_corrupt.push(corruption);
            continue;
        }
        sync_manager.rebuild_projection_local_repo(repo_name)?;
        println!("repair: rebuilt projection for repo {}", repo_name);
        report.rebuilt += 1;
    }
    Ok(report)
}

fn authority_corruption(
    repo_name: &str,
    diagnostic: &deve_core::sync::ProjectionDiagnostic,
) -> AuthorityCorruption {
    let (code, detail) = diagnostic
        .issue
        .as_ref()
        .map(|issue| (issue.code.clone(), issue.detail.clone()))
        .unwrap_or_else(|| {
            (
                "structure_authority".into(),
                "unknown Structure Facts corruption".into(),
            )
        });
    AuthorityCorruption {
        repo_name: repo_name.to_string(),
        code,
        detail,
    }
}

fn print_authority_corruption(corruption: &AuthorityCorruption) {
    println!(
        "repair: projection authority corrupt repo={} code={} detail={}",
        corruption.repo_name, corruption.code, corruption.detail
    );
    println!(
        "repair: skip projection rebuild for repo {}; rebuild cannot modify Structure Facts authority",
        corruption.repo_name
    );
}

#[cfg(test)]
mod tests;

//! plan_ref:
//!   - 03_storage#git-ecosystem-coexistence
//!   - 05_diff_logic#git-mirror-lifecycle
//!   - 14_commands#cli-commands
//!
//! Explicit Git import apply. It writes only Source Control pending state;
//! ledger commit authority remains the normal Stage -> Commit workflow.

use super::error::{GitImportApplyError, GitImportApplyResult};
use super::import_plan::{GitImportPlan, GitImportPlanBlocker, GitImportPlanEntry, plan_import};
use crate::ledger::RepoManager;
use candidate::build_pending_entry;
use pending::apply_pending_candidates;
use serde::{Deserialize, Serialize};
use std::path::Path;

mod candidate;
mod pending;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitImportApplyReport {
    pub plan: GitImportPlan,
    pub applied: usize,
    pub skipped: usize,
    pub blockers: Vec<GitImportPlanBlocker>,
}

pub fn apply_import(
    repo: &RepoManager,
    repo_name: &str,
    repo_root: &Path,
) -> GitImportApplyResult<GitImportApplyReport> {
    let plan = plan_import(repo_root)?;
    let mut apply_blockers = Vec::new();
    let mut candidates = Vec::new();
    if plan.blockers.is_empty() {
        for entry in &plan.entries {
            match build_pending_entry(repo, repo_name, repo_root, entry) {
                Ok(candidate) => candidates.push(candidate),
                Err(reason) => {
                    let reason: String = reason.into();
                    apply_blockers.push(GitImportPlanBlocker {
                        path: entry.path.clone(),
                        reason,
                    });
                }
            }
        }
    }

    let mut report = GitImportApplyReport {
        plan,
        applied: 0,
        skipped: 0,
        blockers: apply_blockers,
    };
    if !report.plan.blockers.is_empty() || !report.blockers.is_empty() || candidates.is_empty() {
        return Ok(report);
    }

    let (applied, skipped, blockers) = repo
        .run_on_local_repo(repo_name, |db| {
            apply_pending_candidates(db, &candidates).map_err(anyhow::Error::from)
        })
        .map_err(|err| GitImportApplyError::LocalRepoApply {
            repo_name: repo_name.to_string(),
            message: err.to_string(),
        })?;
    report.applied = applied;
    report.skipped = skipped;
    report.blockers = blockers;
    Ok(report)
}

#[cfg(test)]
mod tests;

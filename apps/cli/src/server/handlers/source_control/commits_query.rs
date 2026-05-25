//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!
//! Source-control commit history and diff queries.

use crate::server::AppState;
use crate::server::repo_scope::ResolvedRepo;
use deve_core::ledger::source_control as ledger_source_control;
use deve_core::source_control::{self, CommitFileDiff, CommitInfo};
use std::sync::Arc;

pub(super) fn list_commit_history(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    limit: u32,
) -> super::service::ScResult<Vec<CommitInfo>> {
    if let Some(peer_id) = &scope.branch {
        return state
            .repo
            .run_on_shadow_repo_by_id(peer_id, &scope.repo_id, |db| {
                ledger_source_control::repair_missing_commit_order_table(db)?;
                ledger_source_control::list_commits(db, limit)
            })
            .map_err(|e| super::errors::map_repo_error(super::errors::ScOp::CommitHistory, e));
    }
    let selector = super::service::selector_from_scope(scope);
    super::service::list_commit_history(state.repo.as_ref(), &selector, limit)
}

pub(super) fn diff_commits(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    commit_a: Option<&str>,
    commit_b: &str,
) -> super::service::ScResult<Vec<CommitFileDiff>> {
    if let Some(peer_id) = &scope.branch {
        return state
            .repo
            .run_on_shadow_repo_by_id(peer_id, &scope.repo_id, |db| {
                source_control::commit_diff::compare_commits(db, commit_a, commit_b)
            })
            .map_err(|e| {
                super::errors::map_repo_error(
                    super::errors::ScOp::CommitDiff(commit_b.to_string()),
                    e,
                )
            });
    }
    let selector = super::service::selector_from_scope(scope);
    super::service::diff_commits(state.repo.as_ref(), &selector, commit_a, commit_b)
}

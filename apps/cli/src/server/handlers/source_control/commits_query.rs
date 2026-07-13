//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!
//! Source-control commit history and diff queries.

use crate::server::AppState;
use crate::server::repo_scope::ResolvedRepo;
use deve_core::ledger::source_control as ledger_source_control;
use deve_core::source_control::{
    self, CommitFileDiff, CommitFileDiffSummary, CommitFileDiffTarget, CommitInfo,
};
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

pub(super) fn diff_commit_summaries(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    commit_a: Option<&str>,
    commit_b: &str,
) -> super::service::ScResult<Vec<CommitFileDiffSummary>> {
    let result = if let Some(peer_id) = &scope.branch {
        state
            .repo
            .run_on_shadow_repo_by_id(peer_id, &scope.repo_id, |db| {
                source_control::commit_diff::compare_commit_summaries(db, commit_a, commit_b)
            })
    } else {
        state.repo.run_on_local_repo(&scope.repo_name, |db| {
            source_control::commit_diff::compare_commit_summaries(db, commit_a, commit_b)
        })
    };
    result.map_err(|error| {
        super::errors::map_repo_error(super::errors::ScOp::CommitDiff(commit_b.to_string()), error)
    })
}

pub(super) fn diff_commit_file(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    commit_a: Option<&str>,
    commit_b: &str,
    target: &CommitFileDiffTarget,
) -> super::service::ScResult<CommitFileDiff> {
    let result = if let Some(peer_id) = &scope.branch {
        state
            .repo
            .run_on_shadow_repo_by_id(peer_id, &scope.repo_id, |db| {
                source_control::commit_diff::compare_commit_file_exact(
                    db, commit_a, commit_b, target,
                )
            })
    } else {
        state.repo.run_on_local_repo(&scope.repo_name, |db| {
            source_control::commit_diff::compare_commit_file_exact(db, commit_a, commit_b, target)
        })
    };
    result.map_err(|error| {
        super::errors::map_repo_error(super::errors::ScOp::CommitDiff(commit_b.to_string()), error)
    })
}

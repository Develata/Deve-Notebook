//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!
//! Source-control read service operations.

use super::super::errors::{self, ScOp};
use deve_core::git_bridge::{GitMirrorRepairReview, build_repair_review, list_records};
use deve_core::ledger::RepoManager;
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo, SourceControlApi};

pub fn list_pending(
    repo: &dyn SourceControlApi,
    selector: &RepoSelector,
) -> super::ScResult<Vec<ChangeEntry>> {
    repo.list_pending_fs_in_repo(selector)
        .map(super::super::present::collapse_rename_candidates)
        .map_err(|e| errors::map_repo_error(ScOp::ListPending, e))
}

pub fn list_changes(
    repo: &dyn SourceControlApi,
    selector: &RepoSelector,
) -> super::ScResult<Vec<ChangeEntry>> {
    repo.list_changes_in_repo(selector)
        .map(super::super::present::collapse_rename_candidates)
        .map_err(|e| errors::map_repo_error(ScOp::ListChanges, e))
}

pub fn diff_doc_target(
    repo: &dyn SourceControlApi,
    selector: &RepoSelector,
    target: &ScPathTarget,
) -> super::ScResult<String> {
    let entries = list_changes(repo, selector)?;
    let resolved = super::resolve_target(&entries, target)?;
    let path = resolved.path.clone();
    repo.diff_doc_path_in_repo(selector, &resolved)
        .map_err(|e| errors::map_repo_error(ScOp::DiffDoc(path), e))
}

pub fn list_commit_history(
    repo: &dyn SourceControlApi,
    selector: &RepoSelector,
    limit: u32,
) -> super::ScResult<Vec<CommitInfo>> {
    repo.list_commits_in_repo(selector, limit)
        .map_err(|e| errors::map_repo_error(ScOp::CommitHistory, e))
}

pub fn diff_commits(
    repo: &dyn SourceControlApi,
    selector: &RepoSelector,
    commit_a_id: Option<&str>,
    commit_b_id: &str,
) -> super::ScResult<Vec<CommitFileDiff>> {
    repo.diff_commits_in_repo(selector, commit_a_id, commit_b_id)
        .map_err(|e| errors::map_repo_error(ScOp::CommitDiff(commit_b_id.to_string()), e))
}

pub fn git_mirror_repair_review(
    repo: &RepoManager,
    selector: &RepoSelector,
) -> super::ScResult<GitMirrorRepairReview> {
    let repo_name = repo
        .resolve_local_repo_name_for_execution(selector.repo_id, selector.repo_name.as_deref())
        .map_err(|e| errors::map_repo_error(ScOp::ListChanges, e))?;
    let records = repo
        .run_on_local_repo(&repo_name, |db| Ok(list_records(db)?))
        .map_err(|e| errors::map_repo_error(ScOp::ListChanges, e))?;
    Ok(build_repair_review(&repo_name, &records))
}

#[cfg(test)]
mod tests;

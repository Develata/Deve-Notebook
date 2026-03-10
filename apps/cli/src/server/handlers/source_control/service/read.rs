use super::super::errors::{self, ScOp};
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};

pub fn list_pending(
    repo: &dyn Repository,
    selector: &RepoSelector,
) -> super::ScResult<Vec<ChangeEntry>> {
    repo.list_pending_fs_in_repo(selector)
        .map(super::super::present::collapse_rename_candidates)
        .map_err(|e| errors::map_repo_error(ScOp::ListPending, e))
}

pub fn list_changes(
    repo: &dyn Repository,
    selector: &RepoSelector,
) -> super::ScResult<Vec<ChangeEntry>> {
    repo.list_changes_in_repo(selector)
        .map(super::super::present::collapse_rename_candidates)
        .map_err(|e| errors::map_repo_error(ScOp::ListChanges, e))
}

pub fn diff_doc_path(
    repo: &dyn Repository,
    selector: &RepoSelector,
    path: &str,
) -> super::ScResult<String> {
    let path = deve_core::utils::path::to_forward_slash(path);
    repo.diff_doc_path_in_repo(selector, &path)
        .map_err(|e| errors::map_repo_error(ScOp::DiffDoc(path), e))
}

pub fn list_commit_history(
    repo: &dyn Repository,
    selector: &RepoSelector,
    limit: u32,
) -> super::ScResult<Vec<CommitInfo>> {
    repo.list_commits_in_repo(selector, limit)
        .map_err(|e| errors::map_repo_error(ScOp::CommitHistory, e))
}

pub fn diff_commits(
    repo: &dyn Repository,
    selector: &RepoSelector,
    commit_a_id: Option<&str>,
    commit_b_id: &str,
) -> super::ScResult<Vec<CommitFileDiff>> {
    repo.diff_commits_in_repo(selector, commit_a_id, commit_b_id)
        .map_err(|e| errors::map_repo_error(ScOp::CommitDiff(commit_b_id.to_string()), e))
}

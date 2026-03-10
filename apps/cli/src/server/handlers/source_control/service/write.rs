use super::super::errors::{self, ScOp};
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::source_control::CommitInfo;
use std::collections::HashSet;

pub fn stage_pending(
    repo: &dyn Repository,
    selector: &RepoSelector,
    path: &str,
) -> super::ScResult<String> {
    let path = deve_core::utils::path::to_forward_slash(path);
    repo.stage_pending_in_repo(selector, &path)
        .map_err(|e| errors::map_repo_error(ScOp::StagePending(path.clone()), e))?;
    Ok(path)
}

pub fn stage_pending_many(
    repo: &dyn Repository,
    selector: &RepoSelector,
    paths: Vec<String>,
) -> super::ScResult<Vec<String>> {
    let paths = normalized_unique_paths(paths);
    for path in &paths {
        repo.stage_pending_in_repo(selector, path)
            .map_err(|e| errors::map_repo_error(ScOp::StagePending(path.clone()), e))?;
    }
    Ok(paths)
}

pub fn discard_pending(
    repo: &dyn Repository,
    selector: &RepoSelector,
    path: &str,
) -> super::ScResult<String> {
    let path = deve_core::utils::path::to_forward_slash(path);
    repo.discard_pending_in_repo(selector, &path)
        .map_err(|e| errors::map_repo_error(ScOp::DiscardPending(path.clone()), e))?;
    Ok(path)
}

pub fn unstage_file(
    repo: &dyn Repository,
    selector: &RepoSelector,
    path: &str,
) -> super::ScResult<String> {
    let path = deve_core::utils::path::to_forward_slash(path);
    repo.unstage_file_in_repo(selector, &path)
        .map_err(|e| errors::map_repo_error(ScOp::Unstage(path.clone()), e))?;
    Ok(path)
}

pub fn unstage_many(
    repo: &dyn Repository,
    selector: &RepoSelector,
    paths: Vec<String>,
) -> super::ScResult<Vec<String>> {
    let paths = normalized_unique_paths(paths);
    for path in &paths {
        repo.unstage_file_in_repo(selector, path)
            .map_err(|e| errors::map_repo_error(ScOp::Unstage(path.clone()), e))?;
    }
    Ok(paths)
}

pub fn commit_staged(
    repo: &dyn Repository,
    selector: &RepoSelector,
    message: &str,
) -> super::ScResult<CommitInfo> {
    repo.commit_staged_in_repo(selector, message)
        .map_err(|e| errors::map_repo_error(ScOp::Commit, e))
}

fn normalized_unique_paths(paths: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .map(|p| deve_core::utils::path::to_forward_slash(&p))
        .filter(|p| !p.is_empty())
        .filter(|p| seen.insert(p.clone()))
        .collect()
}

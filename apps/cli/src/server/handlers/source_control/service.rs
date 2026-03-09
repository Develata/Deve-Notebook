use crate::server::repo_scope::ResolvedRepo;
use anyhow::Result;
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::source_control::{ChangeEntry, CommitInfo};
use std::collections::HashSet;

pub fn selector_from_scope(scope: &ResolvedRepo) -> RepoSelector {
    RepoSelector {
        repo_id: Some(scope.repo_id),
        repo_name: Some(scope.repo_name.clone()),
    }
}

pub fn list_pending(repo: &dyn Repository, selector: &RepoSelector) -> Result<Vec<ChangeEntry>> {
    repo.list_pending_fs_in_repo(selector)
}

pub fn list_changes(repo: &dyn Repository, selector: &RepoSelector) -> Result<Vec<ChangeEntry>> {
    repo.list_changes_in_repo(selector)
}

pub fn diff_doc_path(repo: &dyn Repository, selector: &RepoSelector, path: &str) -> Result<String> {
    let path = deve_core::utils::path::to_forward_slash(path);
    repo.diff_doc_path_in_repo(selector, &path)
}

pub fn stage_pending(repo: &dyn Repository, selector: &RepoSelector, path: &str) -> Result<String> {
    let path = deve_core::utils::path::to_forward_slash(path);
    repo.stage_pending_in_repo(selector, &path)?;
    Ok(path)
}

pub fn stage_pending_many(
    repo: &dyn Repository,
    selector: &RepoSelector,
    paths: Vec<String>,
) -> Result<Vec<String>> {
    let paths = normalized_unique_paths(paths);
    for path in &paths {
        repo.stage_pending_in_repo(selector, path)?;
    }
    Ok(paths)
}

pub fn discard_pending(
    repo: &dyn Repository,
    selector: &RepoSelector,
    path: &str,
) -> Result<String> {
    let path = deve_core::utils::path::to_forward_slash(path);
    repo.discard_pending_in_repo(selector, &path)?;
    Ok(path)
}

pub fn commit_staged(
    repo: &dyn Repository,
    selector: &RepoSelector,
    message: &str,
) -> Result<CommitInfo> {
    repo.commit_staged_in_repo(selector, message)
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

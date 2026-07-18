//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Switcher selector query helpers.

use crate::server::AppState;
use anyhow::{Result, anyhow};
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{PeerId, RepoId};
use std::sync::Arc;

use super::resolution::{fallback_single_remote_repo, unresolved_target_repo_error};

pub(super) fn select_target_repo_by_url(
    state: &Arc<AppState>,
    had_current_repo_hint: bool,
    target_branch: Option<&PeerId>,
    current_repo_name: Option<&str>,
    current_repo_id: Option<RepoId>,
    url: &str,
) -> Result<Option<String>> {
    if let Some(peer_id) = target_branch {
        state.repo.validate_remote_repo_url_coverage(peer_id)?;
    }
    let repos = state.repo.list_repos(target_branch)?;
    let mut matches = Vec::new();
    let mut current_label_matches_target = false;
    for repo_name in &repos {
        let Some(repo_info) = state
            .repo
            .get_repo_info_for(target_branch, Some(repo_name))?
        else {
            let scope = if target_branch.is_some() {
                "remote"
            } else {
                "local"
            };
            return Err(anyhow!(
                "Broken {scope} repo {} while resolving target repository by URL {}: repository metadata not resolved",
                repo_name,
                url
            ));
        };
        current_label_matches_target |= current_repo_name.is_some_and(|current_name| {
            current_name == repo_name || current_name == repo_info.name
        });
        let Some(repo_url) = state.repo.get_repo_url(target_branch, repo_name)? else {
            let scope = if target_branch.is_some() {
                "remote"
            } else {
                "local"
            };
            return Err(anyhow!(
                "Broken {scope} repo {} while resolving target repository by URL {}: repository URL not resolved",
                repo_name,
                url
            ));
        };
        if repo_url == url {
            matches.push(repo_name.clone());
        }
    }
    if matches.len() > 1 {
        let scope = if target_branch.is_some() {
            "remote"
        } else {
            "local"
        };
        return Err(anyhow!(
            "Ambiguous {scope} repository selector for URL: {}",
            url
        ));
    }
    if matches.len() == 1 {
        return Ok(Some(matches.remove(0)));
    }
    if target_branch.is_some() && current_label_matches_target {
        return Err(unresolved_target_repo_error(
            target_branch,
            current_repo_name,
            current_repo_id,
            Some(url),
        ));
    }
    if let Some(selector) = fallback_single_remote_repo(target_branch, &repos) {
        return Ok(Some(selector));
    }
    if had_current_repo_hint {
        return Err(unresolved_target_repo_error(
            target_branch,
            current_repo_name,
            current_repo_id,
            Some(url),
        ));
    }
    Ok((repos.len() == 1).then(|| repos[0].clone()))
}

pub(super) fn select_repo_selector_by_id(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    repo_id: RepoId,
) -> Result<Option<String>> {
    match branch {
        Some(peer_id) => state.repo.find_remote_repo_selector_by_id(peer_id, repo_id),
        None => state.repo.find_local_repo_name_by_id(repo_id),
    }
}
pub(super) fn recover_selector_from_raw_name(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    raw_repo_name: &str,
) -> Result<Option<String>> {
    let Some(branch) = branch else {
        return state
            .repo
            .resolve_local_repo_name_for_execution(None, Some(raw_repo_name))
            .map(Some);
    };
    Ok(state
        .repo
        .find_remote_repo_selector(branch, raw_repo_name)?
        .filter(|selector| selector == raw_repo_name))
}

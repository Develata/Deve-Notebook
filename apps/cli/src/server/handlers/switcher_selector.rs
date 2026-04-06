use crate::server::AppState;
use anyhow::{Result, anyhow};
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{PeerId, RepoId};
use std::sync::Arc;

#[path = "switcher_requested_repo.rs"]
mod requested_repo;
#[path = "switcher_selector_resolution.rs"]
mod resolution;
#[path = "switcher_selector_support.rs"]
mod support;

use resolution::{fallback_single_remote_repo, unresolved_target_repo_error};
use support::{
    can_defer_to_repo_id_for_display_collision, recover_selector_from_raw_name,
    select_repo_selector_by_id, select_target_repo_by_url,
};

pub(super) fn select_target_repo(
    state: &Arc<AppState>,
    had_current_repo_hint: bool,
    current_repo_id: Option<RepoId>,
    current_repo_name: Option<&str>,
    current_repo_url: Option<String>,
    target_branch: Option<&PeerId>,
) -> Result<Option<String>> {
    if let Some(url) = current_repo_url {
        return select_target_repo_by_url(
            state,
            had_current_repo_hint,
            target_branch,
            current_repo_name,
            current_repo_id,
            &url,
        );
    }
    if let Some(repo_name) = current_repo_name {
        let exact_selector = recover_selector_from_raw_name(state, target_branch, repo_name)?;
        if let Some(exact_selector) = exact_selector {
            if let Some(repo_id) = current_repo_id
                && let Some(selector_by_id) =
                    select_repo_selector_by_id(state, target_branch, repo_id)?
                && selector_by_id != exact_selector
            {
                if can_defer_to_repo_id_for_display_collision(state, target_branch, repo_name)? {
                    return Ok(Some(selector_by_id));
                }
                return Err(anyhow!(
                    "Session repo mismatch: expected {}, resolved selector {} for exact repository selector {}",
                    repo_id,
                    selector_by_id,
                    repo_name
                ));
            }
            return Ok(Some(exact_selector));
        }
        if target_branch.is_some() && uuid::Uuid::parse_str(repo_name).is_ok() {
            return Err(anyhow!("Repository UUID not resolved for {}", repo_name));
        }
    }
    if let Some(repo_id) = current_repo_id
        && let Some(selector) = select_repo_selector_by_id(state, target_branch, repo_id)?
    {
        return Ok(Some(selector));
    }
    if let Some(repo_name) = current_repo_name
        && (target_branch.is_none() || uuid::Uuid::parse_str(repo_name).is_ok())
        && let Some(selector) = recover_selector_from_raw_name(state, target_branch, repo_name)?
    {
        return Ok(Some(selector));
    }
    let repos = state.repo.list_repos(target_branch)?;
    if let Some(selector) = fallback_single_remote_repo(target_branch, &repos) {
        return Ok(Some(selector));
    }
    if had_current_repo_hint || current_repo_name.is_some() || current_repo_id.is_some() {
        return Err(unresolved_target_repo_error(
            target_branch,
            current_repo_name,
            current_repo_id,
            None,
        ));
    }
    Ok((repos.len() == 1).then(|| repos[0].clone()))
}

pub(super) fn resolve_requested_repo_name(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    repo_name: &str,
    repo_id: Option<RepoId>,
) -> Result<Option<String>> {
    requested_repo::resolve_requested_repo_name(state, branch, repo_name, repo_id)
}

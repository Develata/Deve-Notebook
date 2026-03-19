use crate::server::AppState;
use anyhow::{Result, anyhow};
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{PeerId, RepoId};
use std::sync::Arc;

pub(super) fn select_target_repo(
    state: &Arc<AppState>,
    had_current_repo_hint: bool,
    current_repo_id: Option<RepoId>,
    current_repo_name: Option<&str>,
    current_repo_url: Option<String>,
    target_branch: Option<&PeerId>,
) -> Result<Option<String>> {
    if let Some(url) = current_repo_url {
        return select_target_repo_by_url(state, had_current_repo_hint, target_branch, &url);
    }
    if let Some(repo_name) = current_repo_name
        && let Some(exact_selector) =
            recover_selector_from_raw_name_for_switch(state, target_branch, repo_name)?
    {
        if let Some(repo_id) = current_repo_id
            && let Some(selector_by_id) = select_repo_selector_by_id(state, target_branch, repo_id)?
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
    if had_current_repo_hint || current_repo_name.is_some() || current_repo_id.is_some() {
        return Ok(None);
    }
    Ok((repos.len() == 1).then(|| repos[0].clone()))
}

pub(super) fn resolve_requested_repo_name(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    repo_name: &str,
    repo_id: Option<RepoId>,
) -> Result<Option<String>> {
    if let Some(exact_selector) =
        recover_selector_from_raw_name_for_switch(state, branch, repo_name)?
    {
        if let Some(repo_id) = repo_id
            && let Some(selector_by_id) = select_repo_selector_by_id(state, branch, repo_id)?
            && selector_by_id != exact_selector
        {
            if can_defer_to_repo_id_for_display_collision(state, branch, repo_name)? {
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
    if let Some(repo_id) = repo_id
        && let Some(selector) = select_repo_selector_by_id(state, branch, repo_id)?
    {
        return Ok(Some(selector));
    }
    let repos = state.repo.list_repos(branch)?;
    Ok(repos
        .contains(&repo_name.to_string())
        .then(|| repo_name.to_string()))
}

fn select_target_repo_by_url(
    state: &Arc<AppState>,
    had_current_repo_hint: bool,
    target_branch: Option<&PeerId>,
    url: &str,
) -> Result<Option<String>> {
    let repos = state.repo.list_repos(target_branch)?;
    let mut matches = Vec::new();
    for repo_name in &repos {
        let Some(repo_url) = state.repo.get_repo_url(target_branch, repo_name)? else {
            continue;
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
    if had_current_repo_hint {
        return Ok(None);
    }
    Ok((repos.len() == 1).then(|| repos[0].clone()))
}

fn select_repo_selector_by_id(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    repo_id: RepoId,
) -> Result<Option<String>> {
    match branch {
        Some(peer_id) => state.repo.find_remote_repo_selector_by_id(peer_id, repo_id),
        None => state.repo.find_local_repo_name_by_id(repo_id),
    }
}

fn recover_selector_from_raw_name(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    raw_repo_name: &str,
) -> Result<Option<String>> {
    let Some(branch) = branch else {
        return match state
            .repo
            .resolve_local_repo_name_for_execution(None, Some(raw_repo_name))
        {
            Ok(selector) => Ok(Some(selector)),
            Err(err) if local_selector_miss(&err) => Ok(None),
            Err(err) => Err(err),
        };
    };
    if let Ok(repo_id) = uuid::Uuid::parse_str(raw_repo_name) {
        return select_repo_selector_by_id(state, Some(branch), repo_id);
    }
    Ok(state
        .repo
        .find_remote_repo_selector(branch, raw_repo_name)?
        .filter(|selector| selector == raw_repo_name))
}

fn recover_selector_from_raw_name_for_switch(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    raw_repo_name: &str,
) -> Result<Option<String>> {
    recover_selector_from_raw_name(state, branch, raw_repo_name)
}

fn can_defer_to_repo_id_for_display_collision(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    raw_repo_name: &str,
) -> Result<bool> {
    if uuid::Uuid::parse_str(raw_repo_name).is_ok() {
        return Ok(false);
    }
    let Some(peer_id) = branch else {
        return Ok(false);
    };
    if state
        .repo
        .find_remote_repo_selector(peer_id, raw_repo_name)?
        .as_deref()
        == Some(raw_repo_name)
    {
        return Ok(false);
    }
    state.repo.has_remote_display_name(peer_id, raw_repo_name)
}

fn local_selector_miss(err: &anyhow::Error) -> bool {
    matches!(
        err.to_string().as_str(),
        detail if detail.contains("Local repo not found for name")
            || detail.contains("No local repositories available")
    )
}

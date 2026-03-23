use crate::server::AppState;
use anyhow::{Result, anyhow};
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{PeerId, RepoId};
use std::sync::Arc;
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
    for repo_name in &repos {
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
pub(super) fn can_defer_to_repo_id_for_display_collision(
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

pub(super) fn unresolved_target_repo_error(
    target_branch: Option<&PeerId>,
    current_repo_name: Option<&str>,
    current_repo_id: Option<RepoId>,
    current_repo_url: Option<&str>,
) -> anyhow::Error {
    let scope = if target_branch.is_some() {
        "Remote"
    } else {
        "Local"
    };
    if let Some(repo_name) = current_repo_name {
        return anyhow!("{scope} repository selector not resolved for {}", repo_name);
    }
    if let Some(repo_id) = current_repo_id {
        return anyhow!("Repository UUID not resolved for {}", repo_id);
    }
    if let Some(url) = current_repo_url {
        return anyhow!("{scope} repository selector not resolved for URL {}", url);
    }
    anyhow!("{scope} repository selector not resolved")
}

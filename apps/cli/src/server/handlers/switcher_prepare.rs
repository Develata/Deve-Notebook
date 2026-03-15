use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::map_repo_scope_error;
use crate::server::session::WsSession;
use anyhow::anyhow;
use deve_core::ledger::database::DatabaseHandle;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

pub(super) struct PreparedRepoSwitch {
    pub repo_name: String,
    pub repo_id: Option<RepoId>,
    pub db: Option<DatabaseHandle>,
}

pub(super) fn validate_branch_target(
    state: &Arc<AppState>,
    ch: &DualChannel,
    peer_id: &Option<String>,
    switch_nonce: Option<u64>,
) -> Option<Option<String>> {
    let Some(pid_str) = peer_id else {
        return Some(None);
    };
    let shadows = match state.repo.list_switchable_shadows_on_disk() {
        Ok(shadows) => shadows,
        Err(err) => {
            ch.send_protocol_error_with_switch_nonce(
                map_repo_scope_error(anyhow::anyhow!("Failed to list shadows: {}", err)),
                switch_nonce,
            );
            return None;
        }
    };
    let local_repos = match state.repo.list_local_repo_names_for_execution() {
        Ok(repos) => repos,
        Err(err) => {
            ch.send_protocol_error_with_switch_nonce(
                map_repo_scope_error(anyhow::anyhow!("Failed to list local repos: {}", err)),
                switch_nonce,
            );
            return None;
        }
    };
    let is_valid_shadow = shadows.iter().any(|p| p.as_str() == pid_str);
    let is_local_repo = local_repos.contains(pid_str);
    if !is_valid_shadow && is_local_repo {
        ch.send_protocol_error_with_switch_nonce(
            ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                format!(
                    "SwitchBranch expects a shadow peer, got local repo selector: {}",
                    pid_str
                ),
            ),
            switch_nonce,
        );
        return None;
    }
    if !is_valid_shadow {
        ch.send_protocol_error_with_switch_nonce(
            ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                format!("Shadow branch not found: {}", pid_str),
            ),
            switch_nonce,
        );
        return None;
    }
    Some(peer_id.clone())
}

pub(super) fn select_target_repo(
    state: &Arc<AppState>,
    had_current_repo_hint: bool,
    current_repo_id: Option<RepoId>,
    current_repo_name: Option<&str>,
    current_repo_url: Option<String>,
    target_branch: Option<&PeerId>,
) -> anyhow::Result<Option<String>> {
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
    if let Some(url) = current_repo_url {
        let mut matches = Vec::new();
        for repo_name in &repos {
            let Some(repo_url) = state.repo.get_repo_url(target_branch, repo_name)? else {
                continue;
            };
            if repo_url == url {
                matches.push(repo_name.clone());
            }
        }
        if target_branch.is_some() && matches.len() > 1 {
            return Err(anyhow!(
                "Ambiguous remote repository selector for URL: {}",
                url
            ));
        }
        return Ok((matches.len() == 1).then(|| matches[0].clone()));
    }
    if had_current_repo_hint || current_repo_name.is_some() || current_repo_id.is_some() {
        return Ok(None);
    }
    Ok((repos.len() == 1).then(|| repos[0].clone()))
}

fn select_repo_selector_by_id(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    repo_id: RepoId,
) -> anyhow::Result<Option<String>> {
    match branch {
        Some(peer_id) => state.repo.find_remote_repo_selector_by_id(peer_id, repo_id),
        None => state.repo.find_local_repo_name_by_id(repo_id),
    }
}

pub(super) fn prepare_repo_switch(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    repo_name: String,
) -> anyhow::Result<PreparedRepoSwitch> {
    let repo_info = state
        .repo
        .get_repo_info_for(branch, Some(&repo_name))?
        .map(|info| info.uuid);
    if repo_info.is_none() {
        let scope = if branch.is_some() { "Remote" } else { "Local" };
        return Err(anyhow!(
            "{scope} repository UUID not resolved for selector: {}",
            repo_name
        ));
    }
    if branch.is_some() {
        let handle = state.repo.open_database(branch, &repo_name)?;
        return Ok(PreparedRepoSwitch {
            repo_name,
            repo_id: repo_info,
            db: Some(handle),
        });
    }
    state.sync_manager.materialize_local_repo(&repo_name)?;
    Ok(PreparedRepoSwitch {
        repo_name,
        repo_id: repo_info,
        db: None,
    })
}

pub(super) fn resolve_requested_repo_name(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    repo_name: &str,
    repo_id: Option<RepoId>,
) -> anyhow::Result<Option<String>> {
    if let Some(repo_id) = repo_id
        && let Some(selector) = select_repo_selector_by_id(state, branch, repo_id)?
    {
        return Ok(Some(selector));
    }
    if let Some(selector) = recover_selector_from_raw_name(state, branch, repo_name)? {
        return Ok(Some(selector));
    }
    let repos = state.repo.list_repos(branch)?;
    Ok(repos
        .contains(&repo_name.to_string())
        .then(|| repo_name.to_string()))
}

fn recover_selector_from_raw_name(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    raw_repo_name: &str,
) -> anyhow::Result<Option<String>> {
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

fn local_selector_miss(err: &anyhow::Error) -> bool {
    matches!(
        err.to_string().as_str(),
        detail if detail.contains("Local repo not found for name")
            || detail.contains("No local repositories available")
    )
}

pub(super) fn commit_session_switch(
    session: &mut WsSession,
    branch: Option<String>,
    prepared: Option<PreparedRepoSwitch>,
    scope_nonce: Option<u64>,
) {
    session.set_scope_nonce(scope_nonce);
    session.clear_sync_binding();
    session.switch_branch(branch);
    match prepared {
        Some(prepared) => {
            session.switch_repo(prepared.repo_name, prepared.repo_id);
            if let Some(handle) = prepared.db {
                session.set_active_db(handle);
                return;
            }
            session.clear_active_db();
        }
        None => {
            session.clear_active_db();
            session.clear_active_repo();
        }
    }
}

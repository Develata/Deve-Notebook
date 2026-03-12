use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
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
) -> Option<Option<String>> {
    let Some(pid_str) = peer_id else {
        return Some(None);
    };
    let shadows = match state.repo.list_shadows_on_disk() {
        Ok(shadows) => shadows,
        Err(err) => {
            ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::RequestFailed,
                format!("Failed to list shadows: {}", err),
            ));
            return None;
        }
    };
    let local_repos = match state.repo.list_repos(None) {
        Ok(repos) => repos,
        Err(err) => {
            ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::RequestFailed,
                format!("Failed to list local repos: {}", err),
            ));
            return None;
        }
    };
    let is_valid_shadow = shadows.iter().any(|p| p.as_str() == pid_str);
    let is_local_repo = local_repos.contains(pid_str);
    if is_local_repo {
        ch.send_protocol_error(ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            format!(
                "SwitchBranch expects a shadow peer, got local repo selector: {}",
                pid_str
            ),
        ));
        return None;
    }
    if !is_valid_shadow {
        ch.send_protocol_error(ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            format!("Shadow branch not found: {}", pid_str),
        ));
        return None;
    }
    Some(peer_id.clone())
}

pub(super) fn select_target_repo(
    state: &Arc<AppState>,
    current_repo_id: Option<RepoId>,
    current_repo_name: Option<&str>,
    current_repo_url: Option<String>,
    target_branch: Option<&PeerId>,
) -> anyhow::Result<Option<String>> {
    let repos = state.repo.list_repos(target_branch)?;
    if let Some(repo_id) = current_repo_id
        && let Some(info) = state
            .repo
            .get_repo_info_for(target_branch, Some(&repo_id.to_string()))?
        && repos.contains(&info.name)
    {
        return Ok(Some(info.name));
    }
    if let Some(url) = current_repo_url {
        for repo_name in &repos {
            if let Ok(Some(repo_url)) = state.repo.get_repo_url(target_branch, repo_name)
                && repo_url == url
            {
                return Ok(Some(repo_name.clone()));
            }
        }
    }
    if let Some(repo_name) = current_repo_name
        && repos.iter().any(|candidate| candidate == repo_name)
    {
        return Ok(Some(repo_name.to_string()));
    }
    Ok((repos.len() == 1).then(|| repos[0].clone()))
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

pub(super) fn commit_session_switch(
    session: &mut WsSession,
    branch: Option<String>,
    prepared: Option<PreparedRepoSwitch>,
) {
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

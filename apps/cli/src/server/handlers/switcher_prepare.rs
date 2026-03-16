use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::map_repo_scope_error;
use crate::server::session::WsSession;
use anyhow::anyhow;
use deve_core::ledger::database::DatabaseHandle;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::RepoId;
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
                map_repo_scope_error(anyhow!("Failed to list shadows: {}", err)),
                switch_nonce,
            );
            return None;
        }
    };
    let local_repos = match state.repo.list_local_repo_names_for_execution() {
        Ok(repos) => repos,
        Err(err) => {
            ch.send_protocol_error_with_switch_nonce(
                map_repo_scope_error(anyhow!("Failed to list local repos: {}", err)),
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

pub(super) fn prepare_repo_switch(
    state: &Arc<AppState>,
    branch: Option<&deve_core::models::PeerId>,
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
            session.clear_active_repo();
            session.clear_active_db();
        }
    }
}

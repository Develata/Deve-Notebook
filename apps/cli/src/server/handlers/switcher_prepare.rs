use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::map_repo_scope_error;
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
            ch.send_protocol_error(map_repo_scope_error(anyhow::anyhow!(
                "Failed to list shadows: {}",
                err
            )));
            return None;
        }
    };
    let local_repos = match state.repo.list_local_repo_names_for_execution() {
        Ok(repos) => repos,
        Err(err) => {
            ch.send_protocol_error(map_repo_scope_error(anyhow::anyhow!(
                "Failed to list local repos: {}",
                err
            )));
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
        && let Some(selector) = select_repo_selector_by_id(state, target_branch, repo_id)?
    {
        return Ok(Some(selector));
    }
    if let Some(repo_name) = current_repo_name
        && let Some(info) = state
            .repo
            .get_repo_info_for(target_branch, Some(repo_name))?
        && (info.name != repo_name || uuid::Uuid::parse_str(repo_name).is_ok())
    {
        return Ok(Some(repo_name.to_string()));
    }
    if let Some(url) = current_repo_url {
        for repo_name in &repos {
            if let Ok(Some(repo_url)) = state.repo.get_repo_url(target_branch, repo_name)
                && repo_url == url
            {
                return Ok(Some(repo_name.clone()));
            }
        }
        return Ok(None);
    }
    if current_repo_name.is_some() || current_repo_id.is_some() {
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
    let repos = state.repo.list_repos(branch)?;
    if let Some(repo_id) = repo_id
        && let Some(selector) = select_repo_selector_by_id(state, branch, repo_id)?
    {
        return Ok(Some(selector));
    }
    Ok(repos
        .contains(&repo_name.to_string())
        .then(|| repo_name.to_string()))
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

#[cfg(test)]
mod tests {
    use super::select_target_repo;
    use crate::server::{AppState, security, tree_state::RepoTreeRegistry};
    use deve_core::config::SyncMode;
    use deve_core::ledger::RepoManager;
    use deve_core::models::PeerId;
    use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tokio::sync::broadcast;

    fn build_state() -> anyhow::Result<Arc<AppState>> {
        let dir = tempdir()?;
        let vault = dir.path().join("vault");
        let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
        repo.set_vault_root(&vault);
        let repo = Arc::new(repo);
        let (tx, _rx) = broadcast::channel(16);
        let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
        Ok(Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
            tx,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                identity_key.peer_id(),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_service: None,
            identity_key,
        }))
    }

    #[test]
    fn select_target_repo_prefers_collision_safe_remote_selector_for_uuid() -> anyhow::Result<()> {
        let state = build_state()?;
        let peer_id = PeerId::new("peer-remote");
        let first = deve_core::ledger::RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "wiki".into(),
            url: Some("urn:test:wiki-a".into()),
        };
        let second = deve_core::ledger::RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "wiki".into(),
            url: Some("urn:test:wiki-b".into()),
        };
        state.repo.ensure_shadow_repo_info(&peer_id, &first)?;
        state.repo.ensure_shadow_repo_info(&peer_id, &second)?;
        let expected = state
            .repo
            .find_remote_repo_selector_by_id(&peer_id, second.uuid)?
            .expect("selector for second wiki repo");

        let selected = select_target_repo(&state, Some(second.uuid), None, None, Some(&peer_id))?
            .expect("selector for second wiki repo");
        assert_eq!(selected, expected);
        Ok(())
    }
}

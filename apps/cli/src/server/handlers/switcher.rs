use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::listing;
use crate::server::session::WsSession;
use deve_core::ledger::listing::RepoListing;
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;

pub async fn handle_switch_branch(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    peer_id: Option<String>,
) {
    tracing::info!("Handle SwitchBranch request: PeerID={:?}", peer_id);

    let mut final_branch = peer_id.clone();
    let mut force_repo_switch = None;

    if let Some(pid_str) = &peer_id {
        let shadows = match state.repo.list_shadows_on_disk() {
            Ok(shadows) => shadows,
            Err(err) => {
                ch.send_protocol_error(ServerError::with_detail(
                    ServerErrorCode::RequestFailed,
                    format!("Failed to list shadows: {}", err),
                ));
                return;
            }
        };
        let is_valid_shadow = shadows.iter().any(|p| p.as_str() == pid_str);
        let local_repos = match state.repo.list_repos(None) {
            Ok(repos) => repos,
            Err(err) => {
                ch.send_protocol_error(ServerError::with_detail(
                    ServerErrorCode::RequestFailed,
                    format!("Failed to list local repos: {}", err),
                ));
                return;
            }
        };
        let is_local_repo = local_repos.contains(pid_str);

        if !is_valid_shadow && !is_local_repo {
            ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                format!("Shadow branch not found: {}", pid_str),
            ));
            return;
        }

        if !is_valid_shadow && is_local_repo {
            tracing::warn!(
                "Suspicious SwitchBranch: '{}' is a Local Repo but not a Shadow. Correcting to Local Mode.",
                pid_str
            );
            final_branch = None;
            force_repo_switch = Some(pid_str.clone());
        }
    }

    let current_repo_url = if force_repo_switch.is_none() {
        if let Some(current_repo) = &session.active_repo {
            state
                .repo
                .get_repo_url(session.active_branch.as_ref(), current_repo)
                .ok()
                .flatten()
        } else {
            None
        }
    } else {
        None
    };

    session.clear_sync_binding();
    session.switch_branch(final_branch.clone());
    tracing::info!("Session ActiveBranch updated to: {:?}", session.active_branch);

    let target_repo = if let Some(forced) = force_repo_switch {
        Some(forced)
    } else {
        let repos = match state.repo.list_repos(session.active_branch.as_ref()) {
            Ok(repos) => repos,
            Err(err) => {
                ch.send_protocol_error(ServerError::with_detail(
                    ServerErrorCode::RequestFailed,
                    format!("Failed to list repos for branch switch: {}", err),
                ));
                return;
            }
        };
        let mut best_match = None;
        if let Some(url) = &current_repo_url {
            for repo_name in &repos {
                if let Ok(Some(r_url)) = state
                    .repo
                    .get_repo_url(session.active_branch.as_ref(), repo_name)
                    && r_url == *url
                {
                    best_match = Some(repo_name.clone());
                    break;
                }
            }
        }
        if best_match.is_none()
            && let Some(first) = repos.first()
        {
            best_match = Some(first.clone());
        }
        best_match
    };

    if let Some(repo_name) = target_repo {
        tracing::info!("Auto-switching to repo: {}", repo_name);
        let repo_info = state
            .repo
            .get_repo_info_for(session.active_branch.as_ref(), Some(&repo_name))
            .ok()
            .flatten();
        session.switch_repo(repo_name.clone(), repo_info.as_ref().map(|info| info.uuid));

        if session.active_branch.is_some() {
            match state
                .repo
                .open_database(session.active_branch.as_ref(), &repo_name)
            {
                Ok(handle) => {
                    session.set_active_db(handle);
                    tracing::info!(
                        "Database locked: {} (readonly: {})",
                        repo_name,
                        session.is_readonly()
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to lock database: {:?}", e);
                    session.clear_active_db();
                    ch.send_protocol_error(ServerError::with_detail(
                        ServerErrorCode::StorageDbLocked,
                        format!("Failed to lock database: {}", e),
                    ));
                    return;
                }
            }
        } else {
            session.clear_active_db();
            if let Err(e) = state.sync_manager.materialize_local_repo(&repo_name) {
                tracing::error!("Failed to materialize local repo {}: {:?}", repo_name, e);
                ch.send_protocol_error(ServerError::with_detail(
                    ServerErrorCode::StoragePersistFailed,
                    format!("Failed to switch repo workspace: {}", e),
                ));
                return;
            }
        }
    } else {
        session.clear_active_db();
        session.clear_active_repo();
    }

    ch.unicast(ServerMessage::BranchSwitched {
        peer_id: final_branch.clone(),
        success: true,
    });
    listing::handle_list_docs(state, ch, session).await;
    listing::handle_list_repos(state, ch, session.active_branch.as_ref()).await;
}

pub async fn handle_switch_repo(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    name: String,
) {
    tracing::info!("Handle SwitchRepo request: Name='{}', CurrentBranch={:?}", name, session.active_branch);

    let branch = session.active_branch.clone();
    let repos = match state.repo.list_repos(branch.as_ref()) {
        Ok(repos) => repos,
        Err(err) => {
            ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::RequestFailed,
                format!("Failed to list repos: {}", err),
            ));
            return;
        }
    };

    if repos.contains(&name) {
        session.clear_sync_binding();
        let repo_info = state
            .repo
            .get_repo_info_for(branch.as_ref(), Some(&name))
            .ok()
            .flatten();
        session.switch_repo(name.clone(), repo_info.as_ref().map(|info| info.uuid));
        tracing::info!("Client switched to repo: {} (Branch: {:?})", name, branch);

        if branch.is_some() {
            match state.repo.open_database(branch.as_ref(), &name) {
                Ok(handle) => {
                    session.set_active_db(handle);
                    tracing::info!(
                        "Database locked: {} (readonly: {})",
                        name,
                        session.is_readonly()
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to lock database: {:?}", e);
                    session.clear_active_db();
                    ch.send_protocol_error(ServerError::with_detail(
                        ServerErrorCode::StorageDbLocked,
                        format!("Failed to lock database: {}", e),
                    ));
                    return;
                }
            }
        } else {
            session.clear_active_db();
            if let Err(e) = state.sync_manager.materialize_local_repo(&name) {
                tracing::error!("Failed to materialize local repo {}: {:?}", name, e);
                ch.send_protocol_error(ServerError::with_detail(
                    ServerErrorCode::StoragePersistFailed,
                    format!("Failed to switch repo workspace: {}", e),
                ));
                return;
            }
        }
        listing::handle_list_docs(state, ch, session).await;
    } else {
        tracing::warn!(
            "Repo switch failed: '{}' not found in branch {:?}. Available: {:?}",
            name,
            branch,
            repos
        );
        ch.send_protocol_error(ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            format!("Repository not found: {}", name),
        ));
    }
}

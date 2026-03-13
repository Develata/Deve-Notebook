use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::listing;
use crate::server::session::WsSession;
#[path = "switcher_payload.rs"]
mod switcher_payload;
#[path = "switcher_prepare.rs"]
mod switcher_prepare;

use deve_core::ledger::listing::RepoListing;
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;

use self::switcher_payload::preload_branch_switch;
use self::switcher_prepare::{
    commit_session_switch, prepare_repo_switch, select_target_repo, validate_branch_target,
};
use deve_core::models::PeerId;

pub async fn handle_switch_branch(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    peer_id: Option<String>,
    switch_nonce: Option<u64>,
) {
    tracing::info!("Handle SwitchBranch request: PeerID={:?}", peer_id);

    let Some(final_branch) = validate_branch_target(state, ch, &peer_id) else {
        return;
    };

    let current_repo_url = if let Some(current_repo) = &session.active_repo {
        state
            .repo
            .get_repo_url(session.active_branch.as_ref(), current_repo)
            .ok()
            .flatten()
    } else {
        None
    };
    let target_branch = final_branch.as_ref().map(deve_core::models::PeerId::new);
    let target_branch_ref = target_branch.as_ref();

    let target_repo = match select_target_repo(
        state,
        session.active_repo_id,
        session.active_repo.as_deref(),
        current_repo_url,
        target_branch_ref,
    ) {
        Ok(repo) => repo,
        Err(err) => {
            ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::RequestFailed,
                format!("Failed to list repos for branch switch: {}", err),
            ));
            return;
        }
    };
    let prepared = match target_repo {
        Some(repo_name) => {
            tracing::info!("Auto-switching to repo: {}", repo_name);
            match prepare_repo_switch(state, target_branch_ref, repo_name.clone()) {
                Ok(prepared) => Some(prepared),
                Err(err) => {
                    let code = if target_branch_ref.is_some() {
                        ServerErrorCode::StorageDbLocked
                    } else {
                        ServerErrorCode::StoragePersistFailed
                    };
                    ch.send_protocol_error(ServerError::with_detail(
                        code,
                        format!("Failed to switch repo: {}", err),
                    ));
                    return;
                }
            }
        }
        None => None,
    };
    let payload = match preload_branch_switch(state, target_branch_ref, prepared.as_ref()) {
        Ok(payload) => payload,
        Err(err) => {
            ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::RequestFailed,
                format!("Failed to preload branch switch view: {}", err),
            ));
            return;
        }
    };
    commit_session_switch(session, final_branch.clone(), prepared);
    tracing::info!(
        "Session ActiveBranch updated to: {:?}",
        session.active_branch
    );
    if let Some(repo_name) = &session.active_repo {
        tracing::info!(
            "Database locked: {} (readonly: {})",
            repo_name,
            session.is_readonly()
        );
    }

    ch.unicast(ServerMessage::BranchSwitched {
        peer_id: final_branch.clone(),
        success: true,
        switch_nonce,
    });
    ch.unicast(ServerMessage::RepoList {
        request_id: None,
        branch: final_branch.clone(),
        repos: payload.repo_list,
    });
    if let Some(repo_view) = payload.repo_view {
        let tree_branch = final_branch.clone().map(PeerId::new);
        ch.unicast(ServerMessage::RepoSwitched {
            branch: final_branch.clone(),
            name: repo_view.repo_name,
            uuid: repo_view.repo_id.to_string(),
            switch_nonce,
        });
        ch.unicast(ServerMessage::DocList {
            request_id: None,
            repo_id: Some(repo_view.repo_id),
            branch: tree_branch.clone(),
            docs: repo_view.docs,
        });
        let delta = state.tree_manager.reset_from_nodes(
            repo_view.repo_id,
            tree_branch.as_ref(),
            repo_view.nodes,
        );
        ch.unicast(ServerMessage::TreeUpdate {
            request_id: None,
            repo_id: Some(repo_view.repo_id),
            branch: tree_branch,
            delta,
        });
    }
}

pub async fn handle_switch_repo(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    name: String,
    switch_nonce: Option<u64>,
) {
    tracing::info!(
        "Handle SwitchRepo request: Name='{}', CurrentBranch={:?}",
        name,
        session.active_branch
    );

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
        let prepared = match prepare_repo_switch(state, branch.as_ref(), name.clone()) {
            Ok(prepared) => prepared,
            Err(err) => {
                let code = if branch.is_some() {
                    ServerErrorCode::StorageDbLocked
                } else {
                    ServerErrorCode::StoragePersistFailed
                };
                ch.send_protocol_error(ServerError::with_detail(
                    code,
                    format!("Failed to switch repo: {}", err),
                ));
                return;
            }
        };
        commit_session_switch(session, branch.map(|peer| peer.to_string()), Some(prepared));
        tracing::info!(
            "Client switched to repo: {} (Branch: {:?})",
            name,
            session.active_branch
        );
        tracing::info!(
            "Database locked: {} (readonly: {})",
            name,
            session.is_readonly()
        );
        listing::handle_list_docs(state, ch, session, None, switch_nonce).await;
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

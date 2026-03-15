use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{map_repo_scope_error, resolve_session_repo};
use crate::server::session::WsSession;
#[path = "switcher_payload.rs"]
mod switcher_payload;
#[path = "switcher_prepare.rs"]
mod switcher_prepare;
#[cfg(test)]
#[path = "switcher_prepare_test.rs"]
mod switcher_prepare_test;

use deve_core::models::RepoId;
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;

use self::switcher_payload::{emit_repo_view, preload_branch_switch, preload_repo_view};
use self::switcher_prepare::{
    commit_session_switch, prepare_repo_switch, select_target_repo, validate_branch_target,
};

pub async fn handle_switch_branch(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    peer_id: Option<String>,
    switch_nonce: Option<u64>,
) {
    tracing::info!("Handle SwitchBranch request: PeerID={:?}", peer_id);

    let Some(final_branch) = validate_branch_target(state, ch, &peer_id, switch_nonce) else {
        return;
    };

    let had_current_repo_hint = session.active_repo.is_some() || session.active_repo_id.is_some();
    let current_scope = resolve_session_repo(state, session).ok();
    let current_repo_url = current_scope.as_ref().and_then(|scope| {
        state
            .repo
            .get_repo_url(scope.branch.as_ref(), &scope.repo_name)
            .ok()
            .flatten()
    });
    let target_branch = final_branch.as_ref().map(deve_core::models::PeerId::new);
    let target_branch_ref = target_branch.as_ref();

    let target_repo = match select_target_repo(
        state,
        had_current_repo_hint,
        current_scope.as_ref().map(|scope| scope.repo_id),
        current_scope.as_ref().map(|scope| scope.repo_name.as_str()),
        current_repo_url,
        target_branch_ref,
    ) {
        Ok(repo) => repo,
        Err(err) => {
            ch.send_protocol_error_with_switch_nonce(
                map_repo_scope_error(anyhow::anyhow!(
                    "Failed to list repos for branch switch: {}",
                    err
                )),
                switch_nonce,
            );
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
                    ch.send_protocol_error_with_switch_nonce(
                        ServerError::with_detail(code, format!("Failed to switch repo: {}", err)),
                        switch_nonce,
                    );
                    return;
                }
            }
        }
        None => None,
    };
    let payload = match preload_branch_switch(state, target_branch_ref, prepared.as_ref()) {
        Ok(payload) => payload,
        Err(err) => {
            ch.send_protocol_error_with_switch_nonce(
                map_repo_scope_error(anyhow::anyhow!(
                    "Failed to preload branch switch view: {}",
                    err
                )),
                switch_nonce,
            );
            return;
        }
    };
    commit_session_switch(session, final_branch.clone(), prepared, switch_nonce);
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
        scope_nonce: Some(session.scope_nonce()),
        repos: payload.repo_list,
    });
    emit_repo_view(
        state,
        ch,
        session,
        final_branch,
        switch_nonce,
        payload.repo_view,
    );
}

pub async fn handle_switch_repo(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    name: String,
    repo_id: Option<RepoId>,
    switch_nonce: Option<u64>,
) {
    tracing::info!(
        "Handle SwitchRepo request: Name='{}', CurrentBranch={:?}",
        name,
        session.active_branch
    );

    let branch = session.active_branch.clone();
    let repo_name =
        match switcher_prepare::resolve_requested_repo_name(state, branch.as_ref(), &name, repo_id)
        {
            Ok(Some(repo_name)) => repo_name,
            Ok(None) => {
                tracing::warn!(
                    "Repo switch failed: '{}' (repo_id={:?}) not found in branch {:?}",
                    name,
                    repo_id,
                    branch
                );
                ch.send_protocol_error_with_switch_nonce(
                    ServerError::with_detail(
                        ServerErrorCode::ScRepoContextInvalid,
                        format!("Repository not found: {}", name),
                    ),
                    switch_nonce,
                );
                return;
            }
            Err(err) => {
                ch.send_protocol_error_with_switch_nonce(
                    map_repo_scope_error(anyhow::anyhow!("Failed to list repos: {}", err)),
                    switch_nonce,
                );
                return;
            }
        };

    let prepared = match prepare_repo_switch(state, branch.as_ref(), repo_name.clone()) {
        Ok(prepared) => prepared,
        Err(err) => {
            let code = if branch.is_some() {
                ServerErrorCode::StorageDbLocked
            } else {
                ServerErrorCode::StoragePersistFailed
            };
            ch.send_protocol_error_with_switch_nonce(
                ServerError::with_detail(code, format!("Failed to switch repo: {}", err)),
                switch_nonce,
            );
            return;
        }
    };
    let repo_view = match preload_repo_view(state, branch.as_ref(), &prepared) {
        Ok(repo_view) => repo_view,
        Err(err) => {
            ch.send_protocol_error_with_switch_nonce(
                map_repo_scope_error(anyhow::anyhow!(
                    "Failed to preload repo switch view: {}",
                    err
                )),
                switch_nonce,
            );
            return;
        }
    };
    commit_session_switch(
        session,
        branch.map(|peer| peer.to_string()),
        Some(prepared),
        switch_nonce,
    );
    tracing::info!(
        "Client switched to repo: {} (Branch: {:?})",
        repo_name,
        session.active_branch
    );
    tracing::info!(
        "Database locked: {} (readonly: {})",
        repo_name,
        session.is_readonly()
    );
    emit_repo_view(
        state,
        ch,
        session,
        session.active_branch.as_ref().map(ToString::to_string),
        switch_nonce,
        Some(repo_view),
    );
}

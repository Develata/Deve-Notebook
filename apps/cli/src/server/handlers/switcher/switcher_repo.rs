//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Repo switch handler and session scope commit.

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{RepoScopeFailure, map_repo_scope_error};
use crate::server::session::WsSession;
use crate::server::shadow_scope;
use deve_core::models::RepoId;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

use super::switcher_error::prepare_switch_error;
use super::switcher_payload::{
    emit_repo_view, preload_repo_view, prepare_repo_view_messages, switch_scope_nonce,
};
use super::switcher_prepare::{commit_session_switch, prepare_repo_switch};
use super::switcher_selector::resolve_requested_repo_name;

pub(super) async fn handle_switch_repo(
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
    if !super::switcher_guard::require_browser_switch_nonce(
        ch,
        session,
        switch_nonce,
        "repo switch",
    ) {
        return;
    }

    let branch = session.active_branch.clone();
    let repo_name = match resolve_requested_repo_name(state, branch.as_ref(), &name, repo_id) {
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
            let error = map_requested_repo_resolution_error(err);
            if shadow_scope::should_clear_missing_remote_branch(&error) {
                shadow_scope::clear_stale_remote_branch(session);
            }
            ch.send_protocol_error_with_switch_nonce(error, switch_nonce);
            return;
        }
    };

    let prepared = match prepare_repo_switch(state, branch.as_ref(), repo_name.clone()) {
        Ok(prepared) => prepared,
        Err(err) => {
            ch.send_protocol_error_with_switch_nonce(
                prepare_switch_error(branch.as_ref(), err),
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
    let scope_nonce = switch_scope_nonce(session, switch_nonce);
    let repo_view = match prepare_repo_view_messages(
        state,
        branch.as_ref().map(ToString::to_string),
        None,
        scope_nonce,
        switch_nonce,
        Some(repo_view),
    ) {
        Ok(repo_view) => repo_view,
        Err(error) => {
            ch.send_protocol_error_with_switch_nonce(map_repo_scope_error(error), switch_nonce);
            return;
        }
    };
    if let Some(auth_session_id) = session.auth_session_id() {
        state
            .source_control_write_grants()
            .revoke_session(auth_session_id);
    }
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
    emit_repo_view(ch, repo_view);
}

fn map_requested_repo_resolution_error(err: anyhow::Error) -> ServerError {
    if let Some(error) = RepoScopeFailure::from_anyhow(&err)
        && error.is_remote_branch_unavailable()
    {
        return ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, error.detail());
    }
    map_repo_scope_error(anyhow::anyhow!("Failed to list repos: {}", err))
}

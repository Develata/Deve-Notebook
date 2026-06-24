//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Branch switch handler and scope transition orchestration.

use super::switcher_branch_hint::build_branch_switch_selector_input;
use super::switcher_payload::{
    preload_branch_switch, prepare_repo_view_messages, switch_scope_nonce,
};
use super::switcher_prepare::{commit_session_switch, validate_branch_target};
use super::switcher_scope;
use super::switcher_selector::select_target_repo;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::map_repo_scope_error;
use crate::server::session::WsSession;
use std::sync::Arc;

mod emit;
mod prepare;

pub(super) async fn handle_switch_branch(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    peer_id: Option<String>,
    switch_nonce: Option<u64>,
) {
    tracing::info!("Handle SwitchBranch request: PeerID={:?}", peer_id);
    if !super::switcher_guard::require_browser_switch_nonce(
        ch,
        session,
        switch_nonce,
        "branch switch",
    ) {
        return;
    }
    let Some(final_branch) = validate_branch_target(state, ch, &peer_id, switch_nonce) else {
        return;
    };
    let raw_current_repo_hint = session.active_repo.is_some() || session.active_repo_id.is_some();
    let current = match switcher_scope::resolve_current_branch_switch_context(state, session) {
        Ok(current) => current,
        Err(err) => {
            if switcher_scope::clear_failed_current_scope(session, &err) {
                switcher_scope::revoke_source_control_write_grant(state, session);
            }
            ch.send_protocol_error_with_switch_nonce(err, switch_nonce);
            return;
        }
    };
    let target_branch = final_branch
        .as_ref()
        .map(|branch| deve_core::models::PeerId::new(branch.as_str()));
    let target_branch_ref = target_branch.as_ref();
    let selector_input = build_branch_switch_selector_input(
        state,
        session,
        raw_current_repo_hint,
        &current,
        target_branch_ref,
    );
    let target_repo = match select_target_repo(
        state,
        selector_input.had_current_repo_hint,
        selector_input.current_repo_id,
        selector_input.current_repo_name.as_deref(),
        selector_input.current_repo_url,
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
    let prepared = match prepare::prepare_target_repo_switch(state, target_branch_ref, target_repo)
    {
        Ok(prepared) => prepared,
        Err(error) => {
            ch.send_protocol_error_with_switch_nonce(error, switch_nonce);
            return;
        }
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
    let scope_nonce = switch_scope_nonce(session, switch_nonce);
    let repo_view = match prepare_repo_view_messages(
        state,
        final_branch.clone(),
        None,
        scope_nonce,
        switch_nonce,
        payload.repo_view,
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
    commit_session_switch(session, final_branch.clone(), prepared, switch_nonce);
    emit::emit_branch_switch_messages(
        ch,
        final_branch,
        scope_nonce,
        switch_nonce,
        payload.repo_list,
        payload.repo_entries,
        repo_view,
    );
}

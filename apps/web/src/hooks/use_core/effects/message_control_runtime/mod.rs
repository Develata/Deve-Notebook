//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use super::super::state::CoreSignals;
use super::message_control_runtime_repo::{
    clear_repo_scoped_runtime, request_repo_list, request_repo_sync_state,
};
use super::message_shadow;
use crate::api::WsService;
use leptos::prelude::Set;

pub fn refresh_after_branch_switch(
    switch_nonce: Option<u64>,
    ws: &WsService,
    signals: CoreSignals,
) {
    if let Some(switch_nonce) = switch_nonce {
        signals.set_current_scope_nonce.set(switch_nonce);
    }
    ws.clear_writer_ready();
    signals.set_handshake_ready.set(false);
    signals.set_pending_repo_switch.set(None);
    signals.set_current_repo.set(None);
    signals.set_current_repo_id.set(None);
    signals.set_current_doc.set(None);
    signals.set_docs.set(Vec::new());
    signals.set_tree_nodes.set(Vec::new());
    signals.set_repo_list.set(Vec::new());
    clear_repo_scoped_runtime(signals);
    request_repo_list(ws, signals);
}

pub fn refresh_after_repo_switch(ws: &WsService, signals: CoreSignals) {
    ws.clear_writer_ready();
    signals.set_handshake_ready.set(false);
    signals.set_docs.set(Vec::new());
    signals.set_tree_nodes.set(Vec::new());
    clear_repo_scoped_runtime(signals);
    request_repo_list(ws, signals);
    request_repo_sync_state(ws, signals);
    message_shadow::request_shadow_list(ws, signals);
}

#[cfg(test)]
mod tests;

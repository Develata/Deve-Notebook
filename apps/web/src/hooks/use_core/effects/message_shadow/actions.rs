//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::state::CoreSignals;
use crate::hooks::use_core::switch_nonce::next_switch_nonce_after;
use deve_core::protocol::ClientMessage;
use leptos::prelude::{GetUntracked, Set};

pub fn request_shadow_list(ws: &WsService, signals: CoreSignals) {
    let request_id = uuid::Uuid::new_v4().to_string();
    signals
        .set_shadow_list_request_id
        .set(Some(request_id.clone()));
    ws.send(ClientMessage::ListShadows {
        request_id,
        scope_nonce: Some(signals.current_scope_nonce.get_untracked()),
    });
}

pub fn recover_local_branch(ws: &WsService, signals: CoreSignals) {
    let Some(switch_nonce) = next_switch_nonce_after(signals.current_scope_nonce.get_untracked())
    else {
        return;
    };
    ws.clear_writer_ready();
    signals.set_handshake_ready.set(false);
    signals
        .set_pending_branch_switch
        .set(Some(PendingBranchTarget::Local));
    signals
        .set_pending_branch_switch_nonce
        .set(Some(switch_nonce));
    signals.set_pending_repo_switch.set(None);
    signals.set_pending_repo_switch_nonce.set(None);
    ws.send(ClientMessage::SwitchBranch {
        peer_id: None,
        switch_nonce: Some(switch_nonce),
    });
}

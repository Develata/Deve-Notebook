use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::{GetUntracked, Set, Update};

use super::super::state::CoreSignals;
use super::super::switch_nonce::next_switch_nonce;
#[path = "message_shadow_logic.rs"]
mod logic;
pub(crate) use self::logic::{
    should_recover_local_branch_from_deleted_peer, should_recover_local_branch_from_shadow_list,
    should_refresh_shadow_list,
};

#[cfg(test)]
#[path = "message_shadow_test.rs"]
mod tests;

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

pub fn handle_shadow_list(
    shadows: Vec<String>,
    authoritative_refresh: bool,
    ws: &WsService,
    signals: CoreSignals,
) {
    let should_recover = should_recover_local_branch_from_shadow_list(
        &shadows,
        signals.active_branch.get_untracked(),
        signals.pending_branch_switch.get_untracked(),
        signals.pending_repo_switch.get_untracked(),
        authoritative_refresh,
    );
    signals.set_shadow_repos.set(shadows);
    if should_recover {
        recover_local_branch(ws, signals);
    }
}

pub fn handle_peer_deleted(peer_id: String, ws: &WsService, signals: CoreSignals) {
    let deleted_peer = PeerId::new(&peer_id);
    signals.set_peers.update(|peers| {
        peers.remove(&deleted_peer);
    });
    if should_recover_local_branch_from_deleted_peer(
        &deleted_peer,
        signals.active_branch.get_untracked(),
        signals.pending_branch_switch.get_untracked(),
        signals.pending_repo_switch.get_untracked(),
    ) {
        recover_local_branch(ws, signals);
        return;
    }
    if should_refresh_shadow_list(
        signals.pending_branch_switch.get_untracked(),
        signals.pending_repo_switch.get_untracked(),
        signals.shadow_list_request_id.get_untracked().is_some(),
    ) {
        request_shadow_list(ws, signals);
    }
}

fn recover_local_branch(ws: &WsService, signals: CoreSignals) {
    ws.clear_writer_ready();
    signals.set_handshake_ready.set(false);
    let switch_nonce = next_switch_nonce();
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

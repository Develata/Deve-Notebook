use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::{GetUntracked, Set, Update};

use super::super::state::CoreSignals;
use super::super::switch_nonce::next_switch_nonce;

#[cfg(test)]
#[path = "message_shadow_test.rs"]
mod tests;

pub fn request_shadow_list(ws: &WsService, signals: CoreSignals) {
    let request_id = uuid::Uuid::new_v4().to_string();
    signals
        .set_shadow_list_request_id
        .set(Some(request_id.clone()));
    ws.send(ClientMessage::ListShadows { request_id });
}

pub fn handle_shadow_list(shadows: Vec<String>, ws: &WsService, signals: CoreSignals) {
    let should_recover = should_recover_local_branch_from_shadow_list(
        &shadows,
        signals.active_branch.get_untracked(),
        signals.pending_branch_switch.get_untracked(),
    );
    signals.set_shadow_repos.set(shadows);
    if should_recover {
        recover_local_branch(ws, signals);
    }
}

pub fn handle_peer_deleted(peer_id: String, ws: &WsService, signals: CoreSignals) {
    signals.set_peers.update(|peers| {
        peers.remove(&PeerId::new(&peer_id));
    });
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

fn should_refresh_shadow_list(
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
    has_inflight_shadow_list: bool,
) -> bool {
    pending_branch_switch.is_none() && pending_repo_switch.is_none() && !has_inflight_shadow_list
}

// Invariant: 只有 authoritative ShadowList 缺失当前 shadow 分支时，前端才允许恢复本地分支。
fn should_recover_local_branch_from_shadow_list(
    shadows: &[String],
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
) -> bool {
    pending_branch_switch.is_none()
        && active_branch
            .as_ref()
            .map(|peer| !shadows.iter().any(|entry| entry == peer.as_str()))
            .unwrap_or(false)
}

use crate::api::WsService;
use deve_core::models::PeerId;
use leptos::prelude::{GetUntracked, Set, Update};

use super::super::state::CoreSignals;
#[path = "message_shadow_actions.rs"]
mod actions;
#[path = "message_shadow_logic.rs"]
mod logic;
pub use self::actions::request_shadow_list;
pub(crate) use self::logic::{
    should_recover_local_branch_from_deleted_peer, should_recover_local_branch_from_shadow_list,
    should_refresh_shadow_list,
};

#[cfg(test)]
#[path = "message_shadow_test.rs"]
mod tests;

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
        actions::recover_local_branch(ws, signals);
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
        actions::recover_local_branch(ws, signals);
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

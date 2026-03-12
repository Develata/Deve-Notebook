use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::{GetUntracked, Set, Update};

use super::super::effects_sc;
use super::super::effects_switch;
use super::super::state::CoreSignals;
use super::message_scope::string_branch_matches_scope;

pub fn handle_branch_switched(
    peer_id: Option<String>,
    success: bool,
    ws: &WsService,
    signals: CoreSignals,
) {
    if effects_switch::handle_branch_switched(
        peer_id,
        success,
        signals.active_branch,
        signals.pending_branch_switch,
        signals.set_pending_branch_switch,
        signals.set_active_branch,
    ) {
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
        request_shadow_list(ws);
    }
}

pub fn handle_repo_switched(
    branch: Option<String>,
    name: String,
    uuid: String,
    ws: &WsService,
    signals: CoreSignals,
) {
    if !string_branch_matches_scope(
        &branch,
        signals.active_branch.get_untracked(),
        signals.pending_branch_switch.get_untracked(),
    ) {
        return;
    }
    ws.clear_writer_ready();
    signals.set_handshake_ready.set(false);
    if effects_switch::handle_repo_switched(
        name,
        uuid,
        crate::hooks::use_core::RepoSwitchSignals {
            current_repo: signals.current_repo,
            current_repo_id: signals.current_repo_id,
            pending_repo_switch: signals.pending_repo_switch,
            set_pending_repo_switch: signals.set_pending_repo_switch,
            set_current_repo: signals.set_current_repo,
            set_current_repo_id: signals.set_current_repo_id,
            set_current_doc: signals.set_current_doc,
        },
    ) {
        signals.set_docs.set(Vec::new());
        signals.set_tree_nodes.set(Vec::new());
        clear_repo_scoped_runtime(signals);
        request_repo_sync_state(ws);
        request_shadow_list(ws);
    }
}

pub fn handle_peer_deleted(peer_id: String, ws: &WsService, signals: CoreSignals) {
    signals
        .set_shadow_repos
        .update(|peers| peers.retain(|entry| entry != &peer_id));
    signals.set_peers.update(|peers| {
        peers.remove(&PeerId::new(&peer_id));
    });
    if should_recover_local_branch(
        &peer_id,
        signals.active_branch.get_untracked(),
        signals.pending_branch_switch.get_untracked(),
    ) {
        ws.clear_writer_ready();
        signals.set_handshake_ready.set(false);
        signals
            .set_pending_branch_switch
            .set(Some(PendingBranchTarget::Local));
        signals.set_pending_repo_switch.set(None);
        ws.send(ClientMessage::SwitchBranch { peer_id: None });
    }
}

fn clear_repo_scoped_runtime(signals: CoreSignals) {
    signals.set_sync_mode.set("auto".to_string());
    signals.set_pending_ops_count.set(0);
    signals.set_pending_ops_previews.set(Vec::new());
    effects_sc::clear_repo_scoped_state(
        signals.set_staged_changes,
        signals.set_unstaged_changes,
        signals.set_commit_history,
        signals.set_diff_content,
        signals.set_commit_diff_result,
    );
}

fn request_repo_sync_state(ws: &WsService) {
    ws.send(ClientMessage::GetSyncMode);
    ws.send(ClientMessage::GetPendingOps);
}

fn request_shadow_list(ws: &WsService) {
    ws.send(ClientMessage::ListShadows);
}

fn should_recover_local_branch(
    deleted_peer: &str,
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
) -> bool {
    pending_branch_switch.is_none()
        && active_branch.as_ref().map(PeerId::as_str) == Some(deleted_peer)
}

#[cfg(test)]
mod tests {
    use super::should_recover_local_branch;
    use crate::hooks::use_core::PendingBranchTarget;
    use deve_core::models::PeerId;

    #[test]
    fn peer_deleted_only_recovers_current_shadow_branch() {
        assert!(should_recover_local_branch(
            "peer-a",
            Some(PeerId::new("peer-a")),
            None,
        ));
        assert!(!should_recover_local_branch(
            "peer-a",
            Some(PeerId::new("peer-b")),
            None,
        ));
        assert!(!should_recover_local_branch(
            "peer-a",
            Some(PeerId::new("peer-a")),
            Some(PendingBranchTarget::Local),
        ));
    }
}

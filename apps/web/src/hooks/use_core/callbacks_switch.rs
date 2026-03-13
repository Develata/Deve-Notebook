use crate::api::WsService;
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::navigation::{NavigationTarget, guard_navigation};
use super::switch_nonce::next_switch_nonce;
use super::types::{PendingBranchTarget, SwitchScopeSignals};

pub struct SwitchCallbacks {
    pub on_switch_branch: Callback<Option<String>>,
    pub on_switch_repo: Callback<String>,
}

pub fn create_switch_callbacks(ws: &WsService, signals: SwitchScopeSignals) -> SwitchCallbacks {
    let ws_branch = ws.clone();
    let on_switch_branch = Callback::new(move |peer_id: Option<String>| {
        let same_branch = signals
            .active_branch
            .get_untracked()
            .as_ref()
            .map(PeerId::as_str)
            == peer_id.as_deref();
        if same_branch {
            return;
        }
        let target_peer = peer_id.clone();
        let ws_branch_action = ws_branch.clone();
        let action = Callback::new(move |_: ()| {
            let switch_nonce = next_switch_nonce();
            let pending = target_peer
                .clone()
                .map(PendingBranchTarget::Shadow)
                .unwrap_or(PendingBranchTarget::Local);
            signals.set_pending_branch_switch.set(Some(pending));
            signals
                .set_pending_branch_switch_nonce
                .set(Some(switch_nonce));
            signals.set_pending_repo_switch.set(None);
            signals.set_pending_repo_switch_nonce.set(None);
            ws_branch_action.send(ClientMessage::SwitchBranch {
                peer_id: target_peer.clone(),
                switch_nonce: Some(switch_nonce),
            });
        });
        let _ = guard_navigation(
            signals.current_doc.get_untracked(),
            &signals.pending_local_edits.get_untracked(),
            signals.set_pending_navigation,
            NavigationTarget::Branch,
            action,
        );
    });

    let ws_repo = ws.clone();
    let on_switch_repo = Callback::new(move |name: String| {
        if signals.current_repo.get_untracked().as_deref() == Some(name.as_str()) {
            return;
        }
        let target_repo = name.clone();
        let ws_repo_action = ws_repo.clone();
        let action = Callback::new(move |_: ()| {
            let switch_nonce = next_switch_nonce();
            signals
                .set_pending_repo_switch
                .set(Some(target_repo.clone()));
            signals
                .set_pending_repo_switch_nonce
                .set(Some(switch_nonce));
            ws_repo_action.send(ClientMessage::SwitchRepo {
                name: target_repo.clone(),
                switch_nonce: Some(switch_nonce),
            });
        });
        let _ = guard_navigation(
            signals.current_doc.get_untracked(),
            &signals.pending_local_edits.get_untracked(),
            signals.set_pending_navigation,
            NavigationTarget::Repo,
            action,
        );
    });

    SwitchCallbacks {
        on_switch_branch,
        on_switch_repo,
    }
}

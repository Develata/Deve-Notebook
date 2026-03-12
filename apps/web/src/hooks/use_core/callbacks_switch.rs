use crate::api::WsService;
use deve_core::models::{DocId, PeerId};
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::navigation::{NavigationTarget, PendingNavigation, guard_navigation};
use super::pending::PendingLocalEdits;
use super::types::PendingBranchTarget;

pub struct SwitchCallbacks {
    pub on_switch_branch: Callback<Option<String>>,
    pub on_switch_repo: Callback<String>,
}

pub fn create_switch_callbacks(
    ws: &WsService,
    current_doc: ReadSignal<Option<DocId>>,
    pending_local_edits: ReadSignal<PendingLocalEdits>,
    set_pending_navigation: WriteSignal<Option<PendingNavigation>>,
    current_repo: ReadSignal<Option<String>>,
    active_branch: ReadSignal<Option<PeerId>>,
    set_pending_branch_switch: WriteSignal<Option<PendingBranchTarget>>,
    set_pending_repo_switch: WriteSignal<Option<String>>,
) -> SwitchCallbacks {
    let ws_branch = ws.clone();
    let on_switch_branch = Callback::new(move |peer_id: Option<String>| {
        let same_branch =
            active_branch.get_untracked().as_ref().map(PeerId::as_str) == peer_id.as_deref();
        if same_branch {
            return;
        }
        let target_peer = peer_id.clone();
        let ws_branch_action = ws_branch.clone();
        let action = Callback::new(move |_: ()| {
            let pending = target_peer
                .clone()
                .map(PendingBranchTarget::Shadow)
                .unwrap_or(PendingBranchTarget::Local);
            set_pending_branch_switch.set(Some(pending));
            set_pending_repo_switch.set(None);
            ws_branch_action.send(ClientMessage::SwitchBranch {
                peer_id: target_peer.clone(),
            });
        });
        let _ = guard_navigation(
            current_doc.get_untracked(),
            &pending_local_edits.get_untracked(),
            set_pending_navigation,
            NavigationTarget::Branch,
            action,
        );
    });

    let ws_repo = ws.clone();
    let on_switch_repo = Callback::new(move |name: String| {
        if current_repo.get_untracked().as_deref() == Some(name.as_str()) {
            return;
        }
        let target_repo = name.clone();
        let ws_repo_action = ws_repo.clone();
        let action = Callback::new(move |_: ()| {
            set_pending_repo_switch.set(Some(target_repo.clone()));
            ws_repo_action.send(ClientMessage::SwitchRepo {
                name: target_repo.clone(),
            });
        });
        let _ = guard_navigation(
            current_doc.get_untracked(),
            &pending_local_edits.get_untracked(),
            set_pending_navigation,
            NavigationTarget::Repo,
            action,
        );
    });

    SwitchCallbacks {
        on_switch_branch,
        on_switch_repo,
    }
}

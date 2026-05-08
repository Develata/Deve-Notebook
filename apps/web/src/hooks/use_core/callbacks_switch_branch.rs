//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::api::WsService;
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::super::navigation::{NavigationTarget, guard_navigation};
use super::super::switch_nonce::next_switch_nonce_after;
use super::super::types::{PendingBranchTarget, SwitchScopeSignals};
use super::{can_start_scope_switch, prepare_scope_switch, show_switch_block};

pub(super) fn build_switch_branch_callback(
    ws: WsService,
    signals: SwitchScopeSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> Callback<Option<String>> {
    Callback::new(move |peer_id: Option<String>| {
        if !can_start_scope_switch(signals) {
            show_switch_block(set_sync_banner, "switch branch", "scope switching");
            return;
        }
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
        let ws_branch_action = ws.clone();
        let action = Callback::new(move |_: ()| {
            let Some(switch_nonce) =
                next_switch_nonce_after(signals.current_scope_nonce.get_untracked())
            else {
                show_switch_block(set_sync_banner, "switch branch", "scope nonce exhausted");
                return;
            };
            let pending = target_peer
                .clone()
                .map(PendingBranchTarget::Shadow)
                .unwrap_or(PendingBranchTarget::Local);
            prepare_scope_switch(&ws_branch_action, signals);
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
    })
}

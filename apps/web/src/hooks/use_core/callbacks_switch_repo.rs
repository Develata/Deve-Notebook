use crate::api::WsService;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::super::navigation::{NavigationTarget, guard_navigation};
use super::super::switch_nonce::next_switch_nonce_after;
use super::super::types::SwitchScopeSignals;
use super::{can_start_scope_switch, prepare_scope_switch};

pub(super) fn build_switch_repo_callback(
    ws: WsService,
    signals: SwitchScopeSignals,
) -> Callback<String> {
    Callback::new(move |name: String| {
        if !can_start_scope_switch(signals) {
            leptos::logging::warn!("忽略仓库切换: 仍有 scope switch 挂起");
            return;
        }
        if signals.current_repo.get_untracked().as_deref() == Some(name.as_str()) {
            return;
        }

        let target_repo = name.clone();
        let ws_repo_action = ws.clone();
        let action = Callback::new(move |_: ()| {
            let switch_nonce = next_switch_nonce_after(signals.current_scope_nonce.get_untracked());
            prepare_scope_switch(&ws_repo_action, signals);
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
    })
}

//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::super::navigation::{NavigationTarget, guard_navigation};
use super::super::switch_nonce::next_switch_nonce_after;
use super::super::types::SwitchScopeSignals;
use super::{can_start_scope_switch, prepare_scope_switch, show_switch_block};

pub(super) fn build_switch_repo_callback(
    ws: WsService,
    signals: SwitchScopeSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> Callback<String> {
    Callback::new(move |name: String| {
        if !can_start_scope_switch(signals) {
            show_switch_block(set_sync_banner, "switch repo", "scope switching");
            return;
        }
        if signals.current_repo.get_untracked().as_deref() == Some(name.as_str()) {
            return;
        }

        let target_repo = name.clone();
        let ws_repo_action = ws.clone();
        let action = Callback::new(move |_: ()| {
            let Some(switch_nonce) =
                next_switch_nonce_after(signals.current_scope_nonce.get_untracked())
            else {
                show_switch_block(set_sync_banner, "switch repo", "scope nonce exhausted");
                return;
            };
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
            signals.current_repo_id.get_untracked().as_deref(),
            signals.current_scope_nonce.get_untracked(),
            &signals.pending_local_edits.get_untracked(),
            signals.set_pending_navigation,
            NavigationTarget::Repo,
            action,
        );
    })
}

pub(super) fn build_create_repo_callback(
    ws: WsService,
    signals: SwitchScopeSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> Callback<String> {
    Callback::new(move |name: String| {
        let target_repo = name.trim().to_string();
        if target_repo.is_empty() {
            show_switch_block(set_sync_banner, "create repo", "empty repository name");
            return;
        }
        if signals.active_branch.get_untracked().is_some() {
            show_switch_block(set_sync_banner, "create repo", "remote branch view");
            return;
        }
        if !can_start_scope_switch(signals) {
            show_switch_block(set_sync_banner, "create repo", "scope switching");
            return;
        }

        let ws_repo_action = ws.clone();
        let action = Callback::new(move |_: ()| {
            let Some(switch_nonce) =
                next_switch_nonce_after(signals.current_scope_nonce.get_untracked())
            else {
                show_switch_block(set_sync_banner, "create repo", "scope nonce exhausted");
                return;
            };
            prepare_scope_switch(&ws_repo_action, signals);
            signals
                .set_pending_repo_switch
                .set(Some(target_repo.clone()));
            signals
                .set_pending_repo_switch_nonce
                .set(Some(switch_nonce));
            ws_repo_action.send(ClientMessage::CreateRepo {
                name: target_repo.clone(),
                switch_nonce: Some(switch_nonce),
            });
        });
        let _ = guard_navigation(
            signals.current_doc.get_untracked(),
            signals.current_repo_id.get_untracked().as_deref(),
            signals.current_scope_nonce.get_untracked(),
            &signals.pending_local_edits.get_untracked(),
            signals.set_pending_navigation,
            NavigationTarget::Repo,
            action,
        );
    })
}

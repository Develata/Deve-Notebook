//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!

use crate::api::WsService;
use crate::hooks::use_core::RepoSwitchRequest;
use crate::hooks::use_core::navigation::{NavigationTarget, guard_navigation};
use crate::hooks::use_core::switch_nonce::next_switch_nonce_after;
use crate::hooks::use_core::types::{PendingRepoSwitch, SwitchScopeSignals};
use crate::hooks::use_core::write_gate_banner::{WriteGateAction, WriteGateReason};
use crate::i18n::Locale;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::super::{can_start_scope_switch, prepare_scope_switch, show_switch_block};

pub(in crate::hooks::use_core::callbacks_switch) fn build_switch_repo_callback(
    ws: WsService,
    locale: RwSignal<Locale>,
    signals: SwitchScopeSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> Callback<RepoSwitchRequest> {
    Callback::new(move |request: RepoSwitchRequest| {
        if !can_start_scope_switch(signals) {
            show_switch_block(
                set_sync_banner,
                locale,
                WriteGateAction::SwitchRepo,
                WriteGateReason::ScopeSwitching,
            );
            return;
        }
        if repo_switch_request_targets_current(&request, signals) {
            return;
        }

        let selector_name = request.selector_name.clone();
        let expected_name = request.expected_name.clone();
        let repo_id = request.repo_id;
        let ws_repo_action = ws.clone();
        let action = Callback::new(move |_: ()| {
            let Some(switch_nonce) =
                next_switch_nonce_after(signals.current_scope_nonce.get_untracked())
            else {
                show_switch_block(
                    set_sync_banner,
                    locale,
                    WriteGateAction::SwitchRepo,
                    WriteGateReason::ScopeNonceExhausted,
                );
                return;
            };
            prepare_scope_switch(&ws_repo_action, signals);
            signals
                .set_pending_repo_switch
                .set(Some(PendingRepoSwitch::switch(
                    expected_name.clone(),
                    switch_nonce,
                )));
            if let Some(repo_id) = repo_id {
                ws_repo_action.send(ClientMessage::SwitchRepoExact {
                    name: selector_name.clone(),
                    repo_id,
                    switch_nonce: Some(switch_nonce),
                });
            } else {
                ws_repo_action.send(ClientMessage::SwitchRepo {
                    name: selector_name.clone(),
                    switch_nonce: Some(switch_nonce),
                });
            }
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

fn repo_switch_request_targets_current(
    request: &RepoSwitchRequest,
    signals: SwitchScopeSignals,
) -> bool {
    if let Some(repo_id) = request.repo_id {
        let repo_id = repo_id.to_string();
        return signals.current_repo_id.get_untracked().as_deref() == Some(repo_id.as_str());
    }

    signals.current_repo.get_untracked().as_deref() == Some(request.expected_name.as_str())
}

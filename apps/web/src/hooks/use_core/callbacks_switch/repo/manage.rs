//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!

use crate::api::WsService;
use crate::hooks::use_core::navigation::{NavigationTarget, guard_navigation};
use crate::hooks::use_core::switch_nonce::next_switch_nonce_after;
use crate::hooks::use_core::types::{PendingRepoSwitch, SwitchScopeSignals};
use crate::hooks::use_core::write_gate_banner::{WriteGateAction, WriteGateReason};
use crate::hooks::use_core::{RepoRemoveRequest, RepoRenameRequest};
use crate::i18n::Locale;
use deve_core::models::RepoId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::super::{can_start_scope_switch, prepare_scope_switch, show_switch_block};

pub(in crate::hooks::use_core::callbacks_switch) fn build_rename_repo_callback(
    ws: WsService,
    locale: RwSignal<Locale>,
    signals: SwitchScopeSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> Callback<RepoRenameRequest> {
    Callback::new(move |request: RepoRenameRequest| {
        let target_repo = request.new_name.trim().to_string();
        if target_repo.is_empty() {
            show_switch_block(
                set_sync_banner,
                locale,
                WriteGateAction::RenameRepo,
                WriteGateReason::EmptyRepositoryName,
            );
            return;
        }
        if signals.active_branch.get_untracked().is_some() {
            show_switch_block(
                set_sync_banner,
                locale,
                WriteGateAction::RenameRepo,
                WriteGateReason::RemoteBranchView,
            );
            return;
        }
        if !can_start_scope_switch(signals) {
            show_switch_block(
                set_sync_banner,
                locale,
                WriteGateAction::RenameRepo,
                WriteGateReason::ScopeSwitching,
            );
            return;
        }

        let targets_current =
            repo_management_targets_current(signals, request.repo_id, &request.current_name);
        let ws_repo_action = ws.clone();
        let action = Callback::new(move |_: ()| {
            let Some(switch_nonce) =
                next_switch_nonce_after(signals.current_scope_nonce.get_untracked())
            else {
                show_switch_block(
                    set_sync_banner,
                    locale,
                    WriteGateAction::RenameRepo,
                    WriteGateReason::ScopeNonceExhausted,
                );
                return;
            };
            if targets_current {
                prepare_scope_switch(&ws_repo_action, signals);
                signals
                    .set_pending_repo_switch
                    .set(Some(PendingRepoSwitch::rename_current(
                        target_repo.clone(),
                        switch_nonce,
                    )));
            }
            ws_repo_action.send(ClientMessage::RenameRepo {
                repo_id: request.repo_id,
                name: target_repo.clone(),
                switch_nonce: Some(switch_nonce),
            });
        });

        if targets_current {
            let _ = guard_navigation(
                signals.current_doc.get_untracked(),
                signals.current_repo_id.get_untracked().as_deref(),
                signals.current_scope_nonce.get_untracked(),
                &signals.pending_local_edits.get_untracked(),
                signals.set_pending_navigation,
                NavigationTarget::Repo,
                action,
            );
        } else {
            action.run(());
        }
    })
}

pub(in crate::hooks::use_core::callbacks_switch) fn build_remove_repo_callback(
    ws: WsService,
    locale: RwSignal<Locale>,
    signals: SwitchScopeSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> Callback<RepoRemoveRequest> {
    Callback::new(move |request: RepoRemoveRequest| {
        if signals.active_branch.get_untracked().is_some() {
            show_switch_block(
                set_sync_banner,
                locale,
                WriteGateAction::RemoveRepo,
                WriteGateReason::RemoteBranchView,
            );
            return;
        }
        if !can_start_scope_switch(signals) {
            show_switch_block(
                set_sync_banner,
                locale,
                WriteGateAction::RemoveRepo,
                WriteGateReason::ScopeSwitching,
            );
            return;
        }

        let targets_current =
            repo_management_targets_current(signals, request.repo_id, &request.current_name);
        if targets_current && request.fallback_name.as_deref().is_none() {
            show_switch_block(
                set_sync_banner,
                locale,
                WriteGateAction::RemoveRepo,
                WriteGateReason::NoFallbackRepository,
            );
            return;
        }

        let ws_repo_action = ws.clone();
        let action = Callback::new(move |_: ()| {
            let Some(switch_nonce) =
                next_switch_nonce_after(signals.current_scope_nonce.get_untracked())
            else {
                show_switch_block(
                    set_sync_banner,
                    locale,
                    WriteGateAction::RemoveRepo,
                    WriteGateReason::ScopeNonceExhausted,
                );
                return;
            };
            if targets_current {
                prepare_scope_switch(&ws_repo_action, signals);
                signals.set_pending_repo_switch.set(
                    request
                        .fallback_name
                        .clone()
                        .map(|name| PendingRepoSwitch::remove_current(name, switch_nonce)),
                );
            }
            ws_repo_action.send(ClientMessage::RemoveRepo {
                repo_id: request.repo_id,
                switch_nonce: Some(switch_nonce),
            });
        });

        if targets_current {
            let _ = guard_navigation(
                signals.current_doc.get_untracked(),
                signals.current_repo_id.get_untracked().as_deref(),
                signals.current_scope_nonce.get_untracked(),
                &signals.pending_local_edits.get_untracked(),
                signals.set_pending_navigation,
                NavigationTarget::Repo,
                action,
            );
        } else {
            action.run(());
        }
    })
}

fn repo_management_targets_current(
    signals: SwitchScopeSignals,
    repo_id: RepoId,
    current_name: &str,
) -> bool {
    let repo_id = repo_id.to_string();
    signals.current_repo_id.get_untracked().as_deref() == Some(repo_id.as_str())
        || signals.current_repo.get_untracked().as_deref() == Some(current_name)
}

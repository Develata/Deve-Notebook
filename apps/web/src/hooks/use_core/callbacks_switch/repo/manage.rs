//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!

use crate::api::WsService;
use crate::hooks::use_core::types::SwitchScopeSignals;
use crate::hooks::use_core::write_gate_banner::{WriteGateAction, WriteGateReason};
use crate::hooks::use_core::{RepoRemoveRequest, RepoRenameRequest};
use crate::i18n::Locale;
use crate::runtime::repo_control_client::RepoControlClient;
use leptos::prelude::*;

use super::super::{can_start_scope_switch, show_switch_block};

pub(in crate::hooks::use_core::callbacks_switch) fn build_rename_repo_callback(
    ws: WsService,
    locale: RwSignal<Locale>,
    signals: SwitchScopeSignals,
    set_sync_banner: WriteSignal<Option<String>>,
    repo_control: RepoControlClient,
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
        let ws_repo_action = ws.clone();
        let repo_control = repo_control.clone();
        let action = Callback::new(move |_: ()| {
            repo_control.set_alias(
                &ws_repo_action,
                super::repo_control_scope(&ws_repo_action, signals),
                request.repo_id,
                target_repo.clone(),
                request.expected_alias_revision,
            );
        });
        action.run(());
    })
}

pub(in crate::hooks::use_core::callbacks_switch) fn build_remove_repo_callback(
    ws: WsService,
    locale: RwSignal<Locale>,
    signals: SwitchScopeSignals,
    set_sync_banner: WriteSignal<Option<String>>,
    repo_control: RepoControlClient,
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

        let ws_repo_action = ws.clone();
        let repo_control = repo_control.clone();
        let action = Callback::new(move |_: ()| {
            repo_control.prepare_remove_repo(
                &ws_repo_action,
                super::repo_control_scope(&ws_repo_action, signals),
                request.repo_id,
            );
        });
        action.run(());
    })
}

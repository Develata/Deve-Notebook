//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!

use crate::api::WsService;
use crate::hooks::use_core::types::SwitchScopeSignals;
use crate::hooks::use_core::write_gate_banner::{WriteGateAction, WriteGateReason};
use crate::hooks::use_core::{RepoRemoveRequest, RepoRenameRequest};
use crate::i18n::Locale;
use crate::runtime::domain::PendingRepoSwitch;
use crate::runtime::repo_control_client::PreparedRemovalExecutionError;
use crate::runtime::repo_control_client::RepoControlClient;
use deve_core::models::RepoId;
use deve_core::protocol::ServerErrorCode;
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
                request.current_name.clone(),
            );
        });
        action.run(());
    })
}

pub(in crate::hooks::use_core::callbacks_switch) fn build_confirm_remove_repo_callback(
    ws: WsService,
    locale: RwSignal<Locale>,
    signals: SwitchScopeSignals,
    set_sync_banner: WriteSignal<Option<String>>,
    repo_control: RepoControlClient,
) -> Callback<RepoId> {
    Callback::new(move |repo_id: RepoId| {
        let Some(preview) = signals.removal_preview.get_untracked() else {
            return;
        };
        if preview.repo_id != repo_id || !can_start_scope_switch(signals) {
            signals.set_removal_preview.set(None);
            show_switch_block(
                set_sync_banner,
                locale,
                WriteGateAction::RemoveRepo,
                WriteGateReason::ScopeSwitching,
            );
            return;
        }
        let Some(switch_nonce) = signals.current_scope_nonce.get_untracked().checked_add(1) else {
            signals.set_removal_preview.set(None);
            set_sync_banner.set(Some(
                crate::i18n::t::server_error::message(
                    locale.get_untracked(),
                    ServerErrorCode::RepoLifecycleInvalidRequest,
                )
                .to_string(),
            ));
            return;
        };
        match repo_control.execute_prepared_removal(
            &ws,
            super::repo_control_scope(&ws, signals),
            repo_id,
            switch_nonce,
        ) {
            Ok(_) => {
                signals
                    .set_pending_repo_switch
                    .set(Some(PendingRepoSwitch::remove_current(
                        preview.display_alias,
                        switch_nonce,
                    )));
                signals.set_removal_preview.set(None);
            }
            Err(error) => {
                signals.set_removal_preview.set(None);
                let code = match error {
                    PreparedRemovalExecutionError::Blocked => {
                        ServerErrorCode::RepoLifecycleRemovalBlocked
                    }
                    PreparedRemovalExecutionError::Missing => {
                        ServerErrorCode::RepoLifecycleConfirmationInvalid
                    }
                    PreparedRemovalExecutionError::ScopeChanged => {
                        ServerErrorCode::RepoLifecycleConfirmationStale
                    }
                };
                set_sync_banner.set(Some(
                    crate::i18n::t::server_error::message(locale.get_untracked(), code).to_string(),
                ));
            }
        }
    })
}

pub(in crate::hooks::use_core::callbacks_switch) fn build_cancel_remove_repo_callback(
    ws: WsService,
    signals: SwitchScopeSignals,
    repo_control: RepoControlClient,
) -> Callback<RepoId> {
    Callback::new(move |repo_id: RepoId| {
        let _ =
            repo_control.cancel_prepared_removal(&super::repo_control_scope(&ws, signals), repo_id);
        signals.set_removal_preview.set(None);
    })
}

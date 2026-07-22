//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate_banner::{WriteGateAction, WriteGateReason, cannot_action};
use crate::hooks::use_core::{RepoRemoveRequest, RepoRenameRequest, RepoSwitchRequest};
use crate::i18n::Locale;
use crate::runtime::repo_control_client::RepoControlClient;
use crate::runtime::repo_control_client::RepoRemovalPresentation;
use deve_core::models::RepoId;
use leptos::prelude::*;

mod branch;
mod repo;
mod scope;

use super::types::SwitchScopeSignals;
pub(super) use scope::{can_start_scope_switch, prepare_scope_switch};

pub struct SwitchCallbacks {
    pub on_switch_branch: Callback<Option<String>>,
    pub on_switch_repo: Callback<RepoSwitchRequest>,
    pub on_create_repo: Callback<String>,
    pub on_rename_repo: Callback<RepoRenameRequest>,
    pub on_remove_repo: Callback<RepoRemoveRequest>,
    pub removal_preview: ReadSignal<Option<RepoRemovalPresentation>>,
    pub on_confirm_remove_repo: Callback<RepoId>,
    pub on_cancel_remove_repo: Callback<RepoId>,
}

pub(super) fn show_switch_block(
    set_sync_banner: WriteSignal<Option<String>>,
    locale: RwSignal<Locale>,
    action: WriteGateAction,
    reason: WriteGateReason,
) {
    let message = cannot_action(locale.get_untracked(), action, reason);
    warn_sync_banner(set_sync_banner, message);
}

pub fn create_switch_callbacks(
    ws: &WsService,
    locale: RwSignal<Locale>,
    signals: SwitchScopeSignals,
    set_sync_banner: WriteSignal<Option<String>>,
    repo_control: RepoControlClient,
) -> SwitchCallbacks {
    let on_switch_branch =
        branch::build_switch_branch_callback(ws.clone(), locale, signals, set_sync_banner);
    let on_switch_repo =
        repo::build_switch_repo_callback(ws.clone(), locale, signals, set_sync_banner);
    let on_create_repo = repo::build_create_repo_callback(
        ws.clone(),
        locale,
        signals,
        set_sync_banner,
        repo_control.clone(),
    );
    let on_rename_repo = repo::build_rename_repo_callback(
        ws.clone(),
        locale,
        signals,
        set_sync_banner,
        repo_control.clone(),
    );
    let on_remove_repo = repo::build_remove_repo_callback(
        ws.clone(),
        locale,
        signals,
        set_sync_banner,
        repo_control.clone(),
    );
    let on_confirm_remove_repo = repo::build_confirm_remove_repo_callback(
        ws.clone(),
        locale,
        signals,
        set_sync_banner,
        repo_control.clone(),
    );
    let on_cancel_remove_repo =
        repo::build_cancel_remove_repo_callback(ws.clone(), signals, repo_control);

    SwitchCallbacks {
        on_switch_branch,
        on_switch_repo,
        on_create_repo,
        on_rename_repo,
        on_remove_repo,
        removal_preview: signals.removal_preview,
        on_confirm_remove_repo,
        on_cancel_remove_repo,
    }
}

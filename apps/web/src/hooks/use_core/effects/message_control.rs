//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::api::WsService;
use leptos::prelude::GetUntracked;

use super::super::effects_switch;
use super::super::state::CoreSignals;
use super::message_control_runtime::{refresh_after_branch_switch, refresh_after_repo_switch};
use super::message_scope::string_branch_matches_scope;

pub fn handle_branch_switched(
    peer_id: Option<String>,
    success: bool,
    switch_nonce: Option<u64>,
    ws: &WsService,
    signals: CoreSignals,
) {
    if effects_switch::handle_branch_switched(
        peer_id,
        success,
        switch_nonce,
        effects_switch::BranchSwitchSignals {
            pending_branch_switch: signals.pending_branch_switch,
            pending_branch_switch_nonce: signals.pending_branch_switch_nonce,
            set_pending_branch_switch: signals.set_pending_branch_switch,
            set_pending_branch_switch_nonce: signals.set_pending_branch_switch_nonce,
            set_active_branch: signals.set_active_branch,
        },
    ) {
        refresh_after_branch_switch(switch_nonce, ws, signals);
    }
}

pub fn handle_repo_switched(
    branch: Option<String>,
    name: String,
    uuid: String,
    switch_nonce: Option<u64>,
    ws: &WsService,
    signals: CoreSignals,
) {
    if !string_branch_matches_scope(
        &branch,
        signals.active_branch.get_untracked(),
        signals.pending_branch_switch.get_untracked(),
    ) {
        leptos::logging::warn!("忽略 RepoSwitched: branch 与当前 scope 不匹配");
        return;
    }
    let outcome = effects_switch::handle_repo_switched(
        name,
        uuid,
        switch_nonce,
        crate::hooks::use_core::RepoSwitchSignals {
            current_repo: signals.current_repo,
            current_repo_id: signals.current_repo_id,
            pending_repo_switch: signals.pending_repo_switch,
            set_pending_repo_switch: signals.set_pending_repo_switch,
            pending_repo_switch_nonce: signals.pending_repo_switch_nonce,
            set_pending_repo_switch_nonce: signals.set_pending_repo_switch_nonce,
            current_scope_nonce: signals.current_scope_nonce,
            set_current_scope_nonce: signals.set_current_scope_nonce,
            set_current_repo: signals.set_current_repo,
            set_current_repo_id: signals.set_current_repo_id,
            set_current_doc: signals.set_current_doc,
        },
    );
    if outcome.should_refresh {
        refresh_after_repo_switch(ws, signals);
    }
}

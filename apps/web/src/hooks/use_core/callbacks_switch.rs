//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate_banner::cannot_action;
use leptos::prelude::*;

#[path = "callbacks_switch_branch.rs"]
mod branch;
#[path = "callbacks_switch_repo.rs"]
mod repo;
#[path = "callbacks_switch_scope.rs"]
mod scope;

use super::types::SwitchScopeSignals;
pub(super) use scope::{can_start_scope_switch, prepare_scope_switch};

pub struct SwitchCallbacks {
    pub on_switch_branch: Callback<Option<String>>,
    pub on_switch_repo: Callback<String>,
}

pub(super) fn show_switch_block(
    set_sync_banner: WriteSignal<Option<String>>,
    action: &str,
    reason: &str,
) {
    let message = cannot_action(action, reason);
    warn_sync_banner(set_sync_banner, message);
}

pub fn create_switch_callbacks(
    ws: &WsService,
    signals: SwitchScopeSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> SwitchCallbacks {
    let on_switch_branch =
        branch::build_switch_branch_callback(ws.clone(), signals, set_sync_banner);
    let on_switch_repo = repo::build_switch_repo_callback(ws.clone(), signals, set_sync_banner);

    SwitchCallbacks {
        on_switch_branch,
        on_switch_repo,
    }
}

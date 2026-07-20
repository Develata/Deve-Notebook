//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!

mod create;
mod manage;
mod switch;

use crate::api::WsService;
use crate::hooks::use_core::types::SwitchScopeSignals;
use crate::runtime::repo_control_client::RepoControlScope;
use leptos::prelude::GetUntracked;

#[cfg(test)]
mod tests;

pub(super) use create::build_create_repo_callback;
pub(super) use manage::{build_remove_repo_callback, build_rename_repo_callback};
pub(super) use switch::build_switch_repo_callback;

fn repo_control_scope(ws: &WsService, signals: SwitchScopeSignals) -> RepoControlScope {
    RepoControlScope::new(
        ws.connection_epoch.get_untracked(),
        signals
            .current_repo_id
            .get_untracked()
            .and_then(|repo_id| repo_id.parse().ok()),
        signals
            .active_branch
            .get_untracked()
            .map(|branch| branch.to_string()),
        signals.current_scope_nonce.get_untracked(),
    )
}

use crate::api::WsService;
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

pub fn create_switch_callbacks(ws: &WsService, signals: SwitchScopeSignals) -> SwitchCallbacks {
    let on_switch_branch = branch::build_switch_branch_callback(ws.clone(), signals);
    let on_switch_repo = repo::build_switch_repo_callback(ws.clone(), signals);

    SwitchCallbacks {
        on_switch_branch,
        on_switch_repo,
    }
}

//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::api::WsService;

use super::state::CoreSignals;
use super::state_callbacks::CoreStateCallbacks;
#[path = "callbacks_build_scope.rs"]
mod scope;
#[path = "callbacks_build_sections.rs"]
mod sections;

pub(super) fn build_callbacks(ws: &WsService, signals: &CoreSignals) -> CoreStateCallbacks {
    let doc = sections::build_doc_callbacks(ws, signals);
    let sync = sections::build_sync_callbacks(ws, signals);
    let sc = sections::build_source_control_callbacks(ws, signals);
    let misc = sections::build_misc_callbacks(ws, signals);
    let switch = sections::build_switch_callbacks(ws, signals);

    CoreStateCallbacks::new(doc, sync, sc, misc, switch)
}

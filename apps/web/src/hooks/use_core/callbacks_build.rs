//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;

use super::state::CoreSignals;
use super::state_callbacks::CoreStateCallbacks;
mod scope;
mod sections;

pub(super) fn build_callbacks(ws: &WsService, signals: &CoreSignals) -> CoreStateCallbacks {
    let doc = sections::build_doc_callbacks(ws, signals);
    let sync = sections::build_sync_callbacks(ws, signals);
    let sc = sections::build_source_control_callbacks(ws, signals);
    let misc = sections::build_misc_callbacks(ws, signals);
    let switch = sections::build_switch_callbacks(ws, signals);

    CoreStateCallbacks::new(doc, sync, sc, misc, switch)
}

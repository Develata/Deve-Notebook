//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::api::WsService;
use leptos::prelude::*;

use super::CoreState;
use super::state::CoreSignals;
use super::state_callbacks::CoreStateCallbacks;
#[path = "state_build_assemble.rs"]
mod assemble;
#[path = "state_build_doc.rs"]
mod doc;
#[path = "state_build_runtime.rs"]
mod runtime;
#[path = "state_build_source_control.rs"]
mod source_control;
#[path = "state_build_sync.rs"]
mod sync;

pub(super) fn build_core_state(
    ws: WsService,
    signals: &CoreSignals,
    status_text: Signal<String>,
    callbacks: CoreStateCallbacks,
) -> CoreState {
    let CoreStateCallbacks {
        doc,
        sync,
        sc,
        misc,
        switch,
    } = callbacks;
    let doc = doc::build_doc_section(signals, &doc);
    let runtime = runtime::build_runtime_section(signals, status_text, &misc);
    let sync = sync::build_sync_section(signals, &sync, &switch);
    let sc = source_control::build_source_control_section(signals, &sc);

    assemble::assemble_core_state(ws, doc, runtime, sync, sc, switch)
}

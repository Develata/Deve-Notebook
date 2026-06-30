//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::i18n::Locale;
use leptos::prelude::*;

use super::state::CoreSignals;
use super::state_callbacks::CoreStateCallbacks;
mod scope;
mod sections;

pub(super) fn build_callbacks(ws: &WsService, signals: &CoreSignals) -> CoreStateCallbacks {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let doc = sections::build_doc_callbacks(ws, signals, locale);
    let sync = sections::build_sync_callbacks(ws, signals, locale);
    let sc = sections::build_source_control_callbacks(ws, signals, locale);
    let misc = sections::build_misc_callbacks(ws, signals, locale);
    let switch = sections::build_switch_callbacks(ws, signals, locale);

    CoreStateCallbacks::new(doc, sync, sc, misc, switch)
}

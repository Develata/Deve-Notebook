//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use deve_core::protocol::ServerMessage;
use leptos::prelude::Callback;

use super::super::effects_sc;
use super::super::state::CoreSignals;
use super::message_runtime_remaining::handle_remaining;

pub fn handle_sc_or_remaining(
    msg: ServerMessage,
    ws: &WsService,
    signals: CoreSignals,
    schedule_refresh: &dyn Fn(),
    external_changes_refresh: Callback<()>,
) {
    let ctx = effects_sc::ScMessageContext::from_core_signals(
        signals,
        schedule_refresh,
        external_changes_refresh,
        ws,
    );
    if !effects_sc::handle_sc_message(&msg, &ctx) {
        handle_remaining(msg, signals);
    }
}

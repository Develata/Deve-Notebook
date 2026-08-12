//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::{ConnectionStatus, WsService};
use crate::runtime::browser_runtime_lifetime::BrowserRuntimeLifetime;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::super::super::types::HandshakeSignals;
use super::reset::reset_disconnected_state;
mod connected;

pub(super) fn run_handshake_cycle(
    ws: &WsService,
    connection_status: ConnectionStatus,
    endpoint: String,
    signals: HandshakeSignals,
    last_mode: &Rc<RefCell<Option<String>>>,
    handshake_attempt: &Rc<Cell<u64>>,
    runtime_lifetime: BrowserRuntimeLifetime,
) {
    if connection_status != ConnectionStatus::Connected {
        reset_disconnected_state(last_mode, ws, signals);
        return;
    }

    connected::run_connected_handshake_cycle(
        ws,
        endpoint,
        signals,
        last_mode,
        handshake_attempt,
        runtime_lifetime,
    );
}

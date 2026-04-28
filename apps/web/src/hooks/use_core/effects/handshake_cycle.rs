//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::api::{ConnectionStatus, WsService};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::super::super::types::HandshakeSignals;
use super::handshake_reset::reset_disconnected_state;
#[path = "handshake_cycle_connected.rs"]
mod connected;

pub(super) fn run_handshake_cycle(
    ws: &WsService,
    connection_status: ConnectionStatus,
    endpoint: String,
    signals: HandshakeSignals,
    last_mode: &Rc<RefCell<Option<String>>>,
    handshake_attempt: &Rc<Cell<u64>>,
) {
    if connection_status != ConnectionStatus::Connected {
        reset_disconnected_state(last_mode, ws, signals);
        return;
    }

    connected::run_connected_handshake_cycle(ws, endpoint, signals, last_mode, handshake_attempt);
}

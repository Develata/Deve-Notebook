use crate::api::WsService;
use crate::hooks::use_core::types::HandshakeSignals;
use deve_core::models::PeerId;
use leptos::prelude::Set;
use std::cell::RefCell;
use std::rc::Rc;

use super::super::handshake_bootstrap::restore_session_scope;
use super::handshake_state::{
    reset_handshake_attempt, set_handshake_scope_nonce_if_changed, suspended_handshake_mode_key,
};

#[derive(Clone)]
pub(super) struct RestoreScopeTarget {
    pub should_restore: bool,
    pub repo_name: Option<String>,
    pub active_repo_id: Option<String>,
    pub branch: Option<PeerId>,
}

pub(super) fn restore_scope_if_needed(
    ws: &WsService,
    signals: HandshakeSignals,
    should_restore: bool,
    repo_name: Option<String>,
    active_repo_id: Option<String>,
    branch: Option<PeerId>,
) {
    if should_restore {
        restore_session_scope(ws, signals, repo_name, active_repo_id, branch);
    }
}

pub(super) fn reset_disconnected_state(
    last_mode: &Rc<RefCell<Option<String>>>,
    ws: &WsService,
    signals: HandshakeSignals,
) {
    *last_mode.borrow_mut() = None;
    ws.clear_writer_ready();
    signals.set_handshake_ready.set(false);
    set_handshake_scope_nonce_if_changed(signals, None);
}

pub(super) fn suspend_current_handshake(
    last_mode: &Rc<RefCell<Option<String>>>,
    ws: &WsService,
    signals: HandshakeSignals,
    endpoint: &str,
    target: RestoreScopeTarget,
) {
    *last_mode.borrow_mut() = Some(suspended_handshake_mode_key(endpoint));
    restore_scope_if_needed(
        ws,
        signals,
        target.should_restore,
        target.repo_name,
        target.active_repo_id,
        target.branch,
    );
    ws.clear_writer_ready();
    signals.set_handshake_ready.set(false);
    set_handshake_scope_nonce_if_changed(signals, None);
}

pub(super) fn reset_scope_mismatch(
    last_mode: &Rc<RefCell<Option<String>>>,
    ws: &WsService,
    signals: HandshakeSignals,
    should_restore: bool,
    repo_name: Option<String>,
    active_repo_id: Option<String>,
    branch: Option<PeerId>,
) {
    *last_mode.borrow_mut() = None;
    restore_scope_if_needed(
        ws,
        signals,
        should_restore,
        repo_name,
        active_repo_id,
        branch,
    );
    reset_handshake_attempt(last_mode, ws, signals);
}

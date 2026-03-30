use crate::api::WsService;
use crate::hooks::use_core::types::HandshakeSignals;
use leptos::prelude::{GetUntracked, Set};
use std::cell::RefCell;
use std::rc::Rc;

#[path = "handshake_state_mode.rs"]
mod mode;
pub(super) use self::mode::{
    handshake_mode_key, restore_bootstrap_key, should_restore_session_scope,
    should_suspend_handshake, suspended_handshake_mode_key,
};

pub(super) fn reset_handshake_attempt(
    last_mode: &Rc<RefCell<Option<String>>>,
    ws: &WsService,
    signals: HandshakeSignals,
) {
    reset_handshake_attempt_state(last_mode, signals);
    ws.clear_writer_ready();
}

pub(super) fn reset_handshake_attempt_state(
    last_mode: &Rc<RefCell<Option<String>>>,
    signals: HandshakeSignals,
) {
    *last_mode.borrow_mut() = None;
    signals.set_repo_list_request_id.set(None);
    signals.set_doc_list_request_id.set(None);
    signals.set_tree_request_id.set(None);
    signals.set_handshake_ready.set(false);
    set_handshake_scope_nonce_if_changed(signals, None);
}

pub(super) fn set_handshake_scope_nonce_if_changed(
    signals: HandshakeSignals,
    scope_nonce: Option<u64>,
) {
    if signals.handshake_scope_nonce.get_untracked() != scope_nonce {
        signals.set_handshake_scope_nonce.set(scope_nonce);
    }
}

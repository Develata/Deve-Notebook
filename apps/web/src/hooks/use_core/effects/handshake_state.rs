use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::types::HandshakeSignals;
use deve_core::models::PeerId;
use leptos::prelude::Set;
use std::cell::RefCell;
use std::rc::Rc;

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
    signals.set_handshake_scope_nonce.set(None);
}

pub(super) fn handshake_mode_key(
    endpoint: &str,
    degraded: Option<()>,
    repo_id: Option<&str>,
    branch: Option<&PeerId>,
) -> Option<String> {
    degraded
        .map(|_| format!("{endpoint}::degraded"))
        .or_else(|| {
            repo_id.map(|repo_id| {
                let branch_key = branch
                    .map(PeerId::to_string)
                    .unwrap_or_else(|| "local".to_string());
                format!("{endpoint}::{repo_id}::{branch_key}")
            })
        })
}

pub(super) fn should_suspend_handshake(
    branch: &Option<PeerId>,
    pending_branch_switch: Option<&PendingBranchTarget>,
    pending_repo_switch: Option<&str>,
) -> bool {
    branch.is_some() || pending_branch_switch.is_some() || pending_repo_switch.is_some()
}

pub(super) fn should_restore_session_scope(
    is_reconnect_bootstrap: bool,
    pending_branch_switch: Option<&PendingBranchTarget>,
    pending_repo_switch: Option<&str>,
) -> bool {
    is_reconnect_bootstrap && pending_branch_switch.is_none() && pending_repo_switch.is_none()
}

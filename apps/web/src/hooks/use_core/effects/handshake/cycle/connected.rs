//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use leptos::prelude::Get;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::super::super::super::types::HandshakeSignals;
use super::super::super::handshake_bootstrap::restore_session_scope;
mod attempt;
use super::super::reset::{RestoreScopeTarget, suspend_current_handshake};
use super::super::state::{
    handshake_mode_key, restore_bootstrap_key, should_restore_session_scope,
    should_suspend_handshake,
};

pub(super) fn run_connected_handshake_cycle(
    ws: &WsService,
    endpoint: String,
    signals: HandshakeSignals,
    last_mode: &Rc<RefCell<Option<String>>>,
    handshake_attempt: &Rc<Cell<u64>>,
) {
    let maybe_mode = signals.degraded.get();
    let maybe_identity = signals.identity.get();
    let active_repo_id = signals.current_repo_id.get();
    let vector = signals.repo_vector.get();
    let repo_name = signals.current_repo.get();
    let branch = signals.active_branch.get();
    let current_scope_nonce = signals.current_scope_nonce.get();
    let pending_branch_switch = signals.pending_branch_switch.get();
    let pending_repo_switch = signals.pending_repo_switch.get();
    let is_reconnect_bootstrap = last_mode.borrow().is_none();
    let should_restore = should_restore_session_scope(
        is_reconnect_bootstrap,
        pending_branch_switch
            .as_ref()
            .map(|pending| pending.target()),
        pending_repo_switch.as_deref(),
    );
    if should_suspend_handshake(
        &branch,
        pending_branch_switch
            .as_ref()
            .map(|pending| pending.target()),
        pending_repo_switch.as_deref(),
    ) {
        suspend_current_handshake(
            last_mode,
            ws,
            signals,
            &endpoint,
            RestoreScopeTarget {
                should_restore,
                repo_name: repo_name.clone(),
                active_repo_id: active_repo_id.clone(),
                branch: branch.clone(),
            },
        );
        return;
    }
    let Some(mode_key) = handshake_mode_key(
        &endpoint,
        maybe_mode.as_ref().map(|_| ()),
        maybe_identity.as_ref().map(|id| id.repo_id.as_str()),
        branch.as_ref(),
        current_scope_nonce,
    ) else {
        let Some(restore_key) = restore_bootstrap_key(
            &endpoint,
            repo_name.as_deref(),
            branch.as_ref(),
            current_scope_nonce,
            should_restore,
            last_mode.borrow().as_deref(),
        ) else {
            return;
        };
        *last_mode.borrow_mut() = Some(restore_key);
        restore_session_scope(
            ws,
            signals,
            repo_name.clone(),
            active_repo_id.clone(),
            branch.clone(),
        );
        return;
    };
    if last_mode.borrow().as_deref() == Some(mode_key.as_str()) {
        return;
    }
    *last_mode.borrow_mut() = Some(mode_key);
    attempt::start_connected_handshake_attempt(attempt::ConnectedHandshakeAttemptInput {
        ws,
        signals,
        last_mode,
        handshake_attempt,
        maybe_mode,
        maybe_identity,
        vector,
        repo_name,
        active_repo_id,
        branch,
        current_scope_nonce,
        should_restore,
    });
}

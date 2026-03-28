use crate::api::{ConnectionStatus, WsService};
use leptos::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::super::types::HandshakeSignals;
use super::handshake_bootstrap::restore_session_scope;
#[path = "handshake_reset.rs"]
mod handshake_reset;
#[path = "handshake_send.rs"]
mod handshake_send;
#[path = "handshake_state.rs"]
mod handshake_state;
use self::handshake_reset::{
    reset_disconnected_state, reset_scope_mismatch, suspend_current_handshake,
};
use self::handshake_send::{HandshakeAttemptCtx, spawn_handshake_attempt};
use self::handshake_state::{
    handshake_mode_key, restore_bootstrap_key, set_handshake_scope_nonce_if_changed,
    should_restore_session_scope, should_suspend_handshake,
};

/// 设置握手 Effect。
pub fn setup(ws: &WsService, signals: HandshakeSignals) {
    let ws_clone = ws.clone();
    let status_signal = ws.status;
    let endpoint_signal = ws.endpoint;
    let last_mode = Rc::new(RefCell::new(None::<String>));
    let handshake_attempt = Rc::new(Cell::new(0u64));

    Effect::new(move |_| {
        // 失败重置会把 handshake_scope_nonce 清回 None；这里显式订阅它，
        // 以便同一 scope 内的握手准备失败后能重新触发一次 attempt。
        let _handshake_retry_gate = signals.handshake_scope_nonce.get();
        if status_signal.get() != ConnectionStatus::Connected {
            reset_disconnected_state(&last_mode, &ws_clone, signals);
            return;
        }

        let ws = ws_clone.clone();
        let maybe_mode = signals.degraded.get();
        let maybe_identity = signals.identity.get();
        let active_repo_id = signals.current_repo_id.get();
        let vector = signals.repo_vector.get();
        let repo_name = signals.current_repo.get();
        let branch = signals.active_branch.get();
        let current_scope_nonce = signals.current_scope_nonce.get();
        let pending_branch_switch = signals.pending_branch_switch.get();
        let pending_repo_switch = signals.pending_repo_switch.get();
        let endpoint = endpoint_signal.get();
        let is_reconnect_bootstrap = last_mode.borrow().is_none();
        let should_restore = should_restore_session_scope(
            is_reconnect_bootstrap,
            pending_branch_switch.as_ref(),
            pending_repo_switch.as_deref(),
        );
        if should_suspend_handshake(
            &branch,
            pending_branch_switch.as_ref(),
            pending_repo_switch.as_deref(),
        ) {
            suspend_current_handshake(
                &last_mode,
                &ws,
                signals,
                &endpoint,
                should_restore,
                repo_name.clone(),
                active_repo_id.clone(),
                branch.clone(),
            );
            return;
        }
        let Some(mode_key) = handshake_mode_key(
            &endpoint,
            maybe_mode.as_ref().map(|_| ()),
            maybe_identity.as_ref().map(|id| id.repo_id.as_str()),
            branch.as_ref(),
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
                &ws,
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
        ws_clone.clear_writer_ready();
        signals.set_handshake_ready.set(false);
        set_handshake_scope_nonce_if_changed(signals, None);
        if let Some(identity) = maybe_identity.as_ref()
            && maybe_mode.is_none()
            && active_repo_id.as_deref() != Some(identity.repo_id.as_str())
        {
            reset_scope_mismatch(
                &last_mode,
                &ws,
                signals,
                should_restore,
                repo_name.clone(),
                active_repo_id.clone(),
                branch.clone(),
            );
            return;
        }
        set_handshake_scope_nonce_if_changed(signals, Some(current_scope_nonce));
        spawn_handshake_attempt(HandshakeAttemptCtx {
            ws: ws.clone(),
            signals,
            maybe_mode,
            maybe_identity,
            vector,
            repo_name,
            active_repo_id,
            branch,
            current_scope_nonce,
            should_restore,
            handshake_attempt: handshake_attempt.clone(),
            failure_last_mode: last_mode.clone(),
        });
    });
}

#[cfg(test)]
#[path = "handshake_test.rs"]
mod tests;

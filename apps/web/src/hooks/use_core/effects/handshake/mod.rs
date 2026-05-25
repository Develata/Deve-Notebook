//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use leptos::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::super::types::HandshakeSignals;
mod cycle;
mod lifecycle;
mod reset;
mod send;
mod state;
use self::cycle::run_handshake_cycle;
use self::lifecycle::mount_foreground_reprobe_listener;
use self::state::reset_handshake_attempt;
#[cfg(test)]
pub(super) fn handshake_mode_key(
    endpoint: &str,
    degraded: Option<()>,
    repo_id: Option<&str>,
    branch: Option<&deve_core::models::PeerId>,
    scope_nonce: u64,
) -> Option<String> {
    self::state::handshake_mode_key(endpoint, degraded, repo_id, branch, scope_nonce)
}
#[cfg(test)]
pub(super) fn restore_bootstrap_key(
    endpoint: &str,
    repo_name: Option<&str>,
    branch: Option<&deve_core::models::PeerId>,
    scope_nonce: u64,
    should_restore: bool,
    last_mode: Option<&str>,
) -> Option<String> {
    self::state::restore_bootstrap_key(
        endpoint,
        repo_name,
        branch,
        scope_nonce,
        should_restore,
        last_mode,
    )
}
#[cfg(test)]
pub(super) fn should_restore_session_scope(
    is_reconnect_bootstrap: bool,
    pending_branch_switch: Option<&crate::hooks::use_core::PendingBranchTarget>,
    pending_repo_switch: Option<&str>,
) -> bool {
    self::state::should_restore_session_scope(
        is_reconnect_bootstrap,
        pending_branch_switch,
        pending_repo_switch,
    )
}
#[cfg(test)]
pub(super) fn should_suspend_handshake(
    branch: &Option<deve_core::models::PeerId>,
    pending_branch_switch: Option<&crate::hooks::use_core::PendingBranchTarget>,
    pending_repo_switch: Option<&str>,
) -> bool {
    self::state::should_suspend_handshake(branch, pending_branch_switch, pending_repo_switch)
}

/// 设置握手 Effect。
pub fn setup(ws: &WsService, signals: HandshakeSignals) {
    let ws_clone = ws.clone();
    let status_signal = ws.status;
    let endpoint_signal = ws.endpoint;
    let last_mode = Rc::new(RefCell::new(None::<String>));
    let handshake_attempt = Rc::new(Cell::new(0u64));
    let last_manual_retry = Rc::new(Cell::new(signals.handshake_retry_nonce.get_untracked()));
    mount_foreground_reprobe_listener(ws.clone(), signals, last_mode.clone());

    Effect::new(move |_| {
        let manual_retry_nonce = signals.handshake_retry_nonce.get();
        if should_reset_manual_retry(last_manual_retry.get(), manual_retry_nonce) {
            last_manual_retry.set(manual_retry_nonce);
            reset_handshake_attempt(&last_mode, &ws_clone, signals);
        }
        // 失败重置会把 handshake_scope_nonce 清回 None；这里显式订阅它，
        // 以便同一 scope 内的握手准备失败后能重新触发一次 attempt。
        let _handshake_retry_gate = signals.handshake_scope_nonce.get();
        run_handshake_cycle(
            &ws_clone,
            status_signal.get(),
            endpoint_signal.get(),
            signals,
            &last_mode,
            &handshake_attempt,
        );
    });
}

fn should_reset_manual_retry(last_seen: u64, current: u64) -> bool {
    current != last_seen
}

#[cfg(test)]
mod tests;

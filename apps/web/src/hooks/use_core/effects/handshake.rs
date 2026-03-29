use crate::api::WsService;
use leptos::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::super::types::HandshakeSignals;
#[path = "handshake_cycle.rs"]
mod handshake_cycle;
#[path = "handshake_reset.rs"]
mod handshake_reset;
#[path = "handshake_send.rs"]
mod handshake_send;
#[path = "handshake_state.rs"]
mod handshake_state;
use self::handshake_cycle::run_handshake_cycle;
#[cfg(test)]
pub(super) fn handshake_mode_key(
    endpoint: &str,
    degraded: Option<()>,
    repo_id: Option<&str>,
    branch: Option<&deve_core::models::PeerId>,
) -> Option<String> {
    self::handshake_state::handshake_mode_key(endpoint, degraded, repo_id, branch)
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
    self::handshake_state::restore_bootstrap_key(
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
    self::handshake_state::should_restore_session_scope(
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
    self::handshake_state::should_suspend_handshake(
        branch,
        pending_branch_switch,
        pending_repo_switch,
    )
}

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

#[cfg(test)]
#[path = "handshake_test.rs"]
mod tests;

//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!   - 09_web_thin_client_ledger#write-readiness
//!
use crate::api::{
    ConnectionStatus, WsService, http_base_from_ws_url, probe_node_role_for_http_base,
};
use crate::hooks::use_core::types::HandshakeSignals;
use leptos::prelude::GetUntracked;
use leptos::task::spawn_local;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::state::reset_handshake_attempt;

pub(super) fn mount_foreground_reprobe_listener(
    ws: WsService,
    signals: HandshakeSignals,
    last_mode: Rc<RefCell<Option<String>>>,
) {
    use wasm_bindgen::{JsCast, closure::Closure};

    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };

    let last_active = Rc::new(Cell::new(current_page_active()));
    let mount_listener = |target: &web_sys::EventTarget, event_name: &str| {
        let ws = ws.clone();
        let last_mode = last_mode.clone();
        let last_active = last_active.clone();
        let callback = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            handle_page_activity_change(&ws, signals, &last_mode, &last_active);
        }) as Box<dyn FnMut(_)>);
        let _ =
            target.add_event_listener_with_callback(event_name, callback.as_ref().unchecked_ref());
        // Web app 每页只持有一个 WsService；这里的监听器随页面生命周期存在。
        callback.forget();
    };

    mount_listener(document.as_ref(), "visibilitychange");
    mount_listener(window.as_ref(), "focus");
    mount_listener(window.as_ref(), "blur");
}

fn handle_page_activity_change(
    ws: &WsService,
    signals: HandshakeSignals,
    last_mode: &Rc<RefCell<Option<String>>>,
    last_active: &Cell<bool>,
) {
    let active = current_page_active();
    if should_force_foreground_reprobe(last_active.get(), active, ws.status.get_untracked()) {
        reset_foreground_reprobe_state(ws, signals, last_mode);
        spawn_node_role_reprobe(ws.clone());
    }
    last_active.set(active);
}

fn reset_foreground_reprobe_state(
    ws: &WsService,
    signals: HandshakeSignals,
    last_mode: &Rc<RefCell<Option<String>>>,
) {
    ws.begin_foreground_reprobe();
    reset_handshake_attempt(last_mode, ws, signals);
}

fn spawn_node_role_reprobe(ws: WsService) {
    let endpoint = ws.endpoint.get_untracked();
    let connection_epoch = ws.connection_epoch.get_untracked();
    if endpoint.trim().is_empty() {
        ws.fail_foreground_node_role_reprobe();
        return;
    }

    let http_base = http_base_from_ws_url(&endpoint);
    spawn_local(async move {
        let result = probe_node_role_for_http_base(http_base).await;
        if ws.endpoint.get_untracked() != endpoint
            || ws.connection_epoch.get_untracked() != connection_epoch
        {
            return;
        }
        match result {
            Some(result) => ws.complete_foreground_node_role_reprobe(
                result.summary,
                result.source_control_git_bridge,
            ),
            None => ws.fail_foreground_node_role_reprobe(),
        }
    });
}

fn current_page_active() -> bool {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return true;
    };
    !document.hidden() && document.has_focus().unwrap_or(true)
}

fn should_force_foreground_reprobe(
    was_active: bool,
    active: bool,
    status: ConnectionStatus,
) -> bool {
    !was_active && active && matches!(status, ConnectionStatus::Connected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::use_core::{PendingBranchSwitch, PendingRepoSwitch};
    use crate::storage::DegradedSyncMode;
    use deve_core::models::VersionVector;
    use leptos::prelude::{Set, signal};

    #[test]
    fn foreground_reprobe_only_runs_on_connected_foreground_transition() {
        assert!(should_force_foreground_reprobe(
            false,
            true,
            ConnectionStatus::Connected,
        ));
        assert!(!should_force_foreground_reprobe(
            true,
            true,
            ConnectionStatus::Connected,
        ));
        assert!(!should_force_foreground_reprobe(
            false,
            false,
            ConnectionStatus::Connected,
        ));
        assert!(!should_force_foreground_reprobe(
            false,
            true,
            ConnectionStatus::Disconnected,
        ));
        assert!(!should_force_foreground_reprobe(
            false,
            true,
            ConnectionStatus::NativeServiceOffline,
        ));
    }

    #[test]
    fn foreground_reprobe_resets_stale_writer_scope_and_node_role() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        ws.set_node_role_for_test("main");
        ws.mark_writer_ready("repo-a", 7, "web-light-peer");

        let signals = test_handshake_signals();
        signals.set_handshake_ready.set(true);
        signals.set_handshake_scope_nonce.set(Some(7));
        let last_mode = Rc::new(RefCell::new(Some("stale-mode".to_string())));

        reset_foreground_reprobe_state(&ws, signals, &last_mode);

        assert!(last_mode.borrow().is_none());
        assert!(signals.handshake_scope_nonce.get_untracked().is_none());
        assert!(!ws.writer_ready_for(Some("repo-a"), Some(7)));
        assert_eq!(ws.node_role.get_untracked(), "");
        assert_eq!(ws.source_control_git_bridge.get_untracked(), "unknown");
        assert!(ws.node_role_probe_failed.get_untracked());
    }

    fn test_handshake_signals() -> HandshakeSignals {
        let (identity, _) = signal(None);
        let (repo_vector, _) = signal(VersionVector::default());
        let (degraded, _) = signal(None::<DegradedSyncMode>);
        let (current_repo, _) = signal(Some("default".to_string()));
        let (current_repo_id, _) = signal(Some("repo-a".to_string()));
        let (current_scope_nonce, _) = signal(7u64);
        let (active_branch, _) = signal(None);
        let (pending_branch_switch, set_pending_branch_switch) =
            signal(None::<PendingBranchSwitch>);
        let (pending_repo_switch, set_pending_repo_switch) = signal(None::<PendingRepoSwitch>);
        let (handshake_scope_nonce, set_handshake_scope_nonce) = signal(None);
        let (handshake_retry_nonce, _) = signal(0u64);
        let (_, set_repo_list_request_id) = signal(None);
        let (_, set_doc_list_request_id) = signal(None);
        let (_, set_tree_request_id) = signal(None);
        let (_, set_handshake_ready) = signal(false);

        HandshakeSignals {
            identity,
            repo_vector,
            degraded,
            current_repo,
            current_repo_id,
            current_scope_nonce,
            active_branch,
            pending_branch_switch,
            set_pending_branch_switch,
            pending_repo_switch,
            set_pending_repo_switch,
            handshake_scope_nonce,
            set_handshake_scope_nonce,
            handshake_retry_nonce,
            set_repo_list_request_id,
            set_doc_list_request_id,
            set_tree_request_id,
            set_handshake_ready,
        }
    }
}

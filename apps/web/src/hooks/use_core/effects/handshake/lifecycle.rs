//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!   - 09_web_thin_client_ledger#write-readiness
//!
use crate::api::{
    AuthProbe, ConnectionStatus, WsService, current_native_bootstrap_blocked_status,
    current_native_platform_lifecycle_authority, http_base_from_ws_url,
    probe_auth_status_with_http_base, probe_node_role_for_http_base,
};
use crate::hooks::use_core::types::HandshakeSignals;
use leptos::prelude::GetUntracked;
use leptos::task::spawn_local;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::state::reset_handshake_attempt;

#[derive(Clone, Copy)]
enum ForegroundReprobeSource {
    PageVisibility,
    PageFocus,
    NativeSuspend,
    NativeResume,
}

impl ForegroundReprobeSource {
    fn category(self) -> &'static str {
        match self {
            Self::PageVisibility => "page-visibility",
            Self::PageFocus => "page-focus",
            Self::NativeSuspend => "native-suspend",
            Self::NativeResume => "native-resume",
        }
    }
}

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

    let native_lifecycle_authority = current_native_platform_lifecycle_authority();
    let last_active = Rc::new(Cell::new(current_page_active()));
    let mount_listener =
        |target: &web_sys::EventTarget, event_name: &str, source: ForegroundReprobeSource| {
            let ws = ws.clone();
            let last_mode = last_mode.clone();
            let last_active = last_active.clone();
            let callback = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                handle_page_activity_change(
                    &ws,
                    signals,
                    &last_mode,
                    &last_active,
                    native_lifecycle_authority,
                    source,
                );
            }) as Box<dyn FnMut(_)>);
            let _ = target
                .add_event_listener_with_callback(event_name, callback.as_ref().unchecked_ref());
            // Web app 每页只持有一个 WsService；这里的监听器随页面生命周期存在。
            callback.forget();
        };

    if page_lifecycle_listeners_enabled(native_lifecycle_authority) {
        mount_listener(
            document.as_ref(),
            "visibilitychange",
            ForegroundReprobeSource::PageVisibility,
        );
        mount_listener(window.as_ref(), "focus", ForegroundReprobeSource::PageFocus);
        mount_listener(window.as_ref(), "blur", ForegroundReprobeSource::PageFocus);
    }

    let mount_native_listener = |event_name: &str, suspended: bool| {
        let ws = ws.clone();
        let last_mode = last_mode.clone();
        let last_active = last_active.clone();
        let callback = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            if suspended {
                handle_native_suspend(&ws, signals, &last_mode, &last_active);
            } else {
                handle_native_resume(&ws, signals, &last_mode, &last_active);
            }
        }) as Box<dyn FnMut(_)>);
        let _ =
            window.add_event_listener_with_callback(event_name, callback.as_ref().unchecked_ref());
        callback.forget();
    };
    mount_native_listener("deve-native-suspended", true);
    mount_native_listener("deve-native-resumed", false);

    let ws_error = ws.clone();
    let error_callback = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        handle_native_service_error(&ws_error, current_native_bootstrap_blocked_status());
    }) as Box<dyn FnMut(_)>);
    let _ = window.add_event_listener_with_callback(
        "deve-native-service-error",
        error_callback.as_ref().unchecked_ref(),
    );
    error_callback.forget();
}

fn handle_native_service_error(ws: &WsService, projected_status: Option<ConnectionStatus>) {
    if projected_status == Some(ConnectionStatus::Unauthorized) {
        ws.mark_unauthorized();
    } else {
        ws.mark_native_service_offline();
    }
}

fn handle_page_activity_change(
    ws: &WsService,
    signals: HandshakeSignals,
    last_mode: &Rc<RefCell<Option<String>>>,
    last_active: &Cell<bool>,
    native_lifecycle_authority: bool,
    source: ForegroundReprobeSource,
) {
    let active = current_page_active();
    if apply_page_activity_transition(
        ws,
        signals,
        last_mode,
        last_active,
        native_lifecycle_authority,
        active,
        source,
    ) {
        spawn_foreground_reprobe(ws.clone());
    }
}

fn apply_page_activity_transition(
    ws: &WsService,
    signals: HandshakeSignals,
    last_mode: &Rc<RefCell<Option<String>>>,
    last_active: &Cell<bool>,
    native_lifecycle_authority: bool,
    active: bool,
    source: ForegroundReprobeSource,
) -> bool {
    if native_lifecycle_authority {
        return false;
    }
    let should_reprobe =
        should_force_foreground_reprobe(last_active.get(), active, ws.status.get_untracked());
    if should_reprobe {
        reset_foreground_reprobe_state(ws, signals, last_mode, source);
    }
    last_active.set(active);
    should_reprobe
}

fn page_lifecycle_listeners_enabled(native_lifecycle_authority: bool) -> bool {
    !native_lifecycle_authority
}

fn reset_foreground_reprobe_state(
    ws: &WsService,
    signals: HandshakeSignals,
    last_mode: &Rc<RefCell<Option<String>>>,
    source: ForegroundReprobeSource,
) {
    leptos::logging::log!(
        "deve_lifecycle_checkpoint category=foreground_reprobe source={} connection_epoch={} scope_nonce={}",
        source.category(),
        ws.connection_epoch.get_untracked(),
        signals.current_scope_nonce.get_untracked(),
    );
    ws.begin_foreground_reprobe();
    reset_handshake_attempt(last_mode, ws, signals);
}

fn handle_native_suspend(
    ws: &WsService,
    signals: HandshakeSignals,
    last_mode: &Rc<RefCell<Option<String>>>,
    last_active: &Cell<bool>,
) {
    if matches!(ws.status.get_untracked(), ConnectionStatus::Connected) {
        reset_foreground_reprobe_state(
            ws,
            signals,
            last_mode,
            ForegroundReprobeSource::NativeSuspend,
        );
    }
    last_active.set(false);
}

fn handle_native_resume(
    ws: &WsService,
    signals: HandshakeSignals,
    last_mode: &Rc<RefCell<Option<String>>>,
    last_active: &Cell<bool>,
) {
    reset_foreground_reprobe_state(
        ws,
        signals,
        last_mode,
        ForegroundReprobeSource::NativeResume,
    );
    ws.request_native_endpoint_rebind();
    last_active.set(true);
}

fn spawn_foreground_reprobe(ws: WsService) {
    let endpoint = ws.endpoint.get_untracked();
    let connection_epoch = ws.connection_epoch.get_untracked();
    if endpoint.trim().is_empty() {
        ws.fail_foreground_node_role_reprobe();
        return;
    }

    let http_base = http_base_from_ws_url(&endpoint);
    spawn_local(async move {
        match probe_auth_status_with_http_base(Some(&http_base)).await {
            AuthProbe::Valid => {}
            AuthProbe::Invalid => {
                ws.mark_unauthorized();
                return;
            }
            AuthProbe::Unknown => {
                ws.fail_foreground_node_role_reprobe();
                return;
            }
        }
        let result = probe_node_role_for_http_base(http_base).await;
        if ws.endpoint.get_untracked() != endpoint
            || ws.connection_epoch.get_untracked() != connection_epoch
        {
            return;
        }
        match result {
            Some(result) => ws.complete_foreground_node_role_reprobe(
                result.summary,
                result.source_control_authority,
                result.host_file_copy_absolute_path,
                result.host_file_reveal_in_system_explorer,
                result.watcher_health,
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
    fn android_keyboard_focus_change_does_not_trigger_foreground_reprobe() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        ws.set_node_role_for_test("main");
        ws.mark_writer_ready("repo-a", 7, "web-light-peer");
        let signals = test_handshake_signals();
        signals.set_handshake_ready.set(true);
        signals.set_handshake_scope_nonce.set(Some(7));
        let last_mode = Rc::new(RefCell::new(Some("ready-mode".to_string())));
        let last_active = Cell::new(false);

        assert!(!page_lifecycle_listeners_enabled(true));
        assert!(!apply_page_activity_transition(
            &ws,
            signals,
            &last_mode,
            &last_active,
            true,
            true,
            ForegroundReprobeSource::PageFocus,
        ));

        assert_eq!(ws.status.get_untracked(), ConnectionStatus::Connected);
        assert!(ws.writer_ready_for(Some("repo-a"), Some(7)));
        assert_eq!(signals.handshake_scope_nonce.get_untracked(), Some(7));
        assert_eq!(last_mode.borrow().as_deref(), Some("ready-mode"));
        assert!(!last_active.get());
        assert!(ws.drain_connection_controls_for_test().is_empty());
    }

    #[test]
    fn browser_page_lifecycle_listeners_remain_enabled() {
        assert!(page_lifecycle_listeners_enabled(false));
    }

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

        reset_foreground_reprobe_state(
            &ws,
            signals,
            &last_mode,
            ForegroundReprobeSource::PageVisibility,
        );

        assert!(last_mode.borrow().is_none());
        assert_eq!(
            ws.status.get_untracked(),
            ConnectionStatus::NativeReprobeRequired
        );
        assert!(signals.handshake_scope_nonce.get_untracked().is_none());
        assert!(!ws.writer_ready_for(Some("repo-a"), Some(7)));
        assert_eq!(ws.node_role.get_untracked(), "");
        assert_eq!(ws.source_control_authority.get_untracked(), "unknown");
        assert!(ws.node_role_probe_failed.get_untracked());
    }

    #[test]
    fn native_suspend_immediately_revokes_write_readiness() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        ws.set_node_role_for_test("main");
        ws.mark_writer_ready("repo-a", 7, "web-light-peer");
        let signals = test_handshake_signals();
        signals.set_handshake_ready.set(true);
        signals.set_handshake_scope_nonce.set(Some(7));
        let last_mode = Rc::new(RefCell::new(Some("ready-mode".to_string())));
        let last_active = Cell::new(true);

        handle_native_suspend(&ws, signals, &last_mode, &last_active);

        assert!(!last_active.get());
        assert_eq!(
            ws.status.get_untracked(),
            ConnectionStatus::NativeReprobeRequired
        );
        assert!(!ws.writer_ready_for(Some("repo-a"), Some(7)));
        assert!(signals.handshake_scope_nonce.get_untracked().is_none());
        assert!(last_mode.borrow().is_none());
    }

    #[test]
    fn native_resume_requests_dynamic_endpoint_rebind_without_old_endpoint_probe() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let ws = WsService::new_for_test(ConnectionStatus::Disconnected);
        let signals = test_handshake_signals();
        let last_mode = Rc::new(RefCell::new(Some("stale-mode".to_string())));
        let last_active = Cell::new(false);

        handle_native_resume(&ws, signals, &last_mode, &last_active);

        assert!(last_active.get());
        assert_eq!(
            ws.status.get_untracked(),
            ConnectionStatus::NativeReprobeRequired
        );
        assert_eq!(ws.drain_connection_controls_for_test().len(), 1);
    }

    #[test]
    fn native_session_invalid_event_uses_typed_unauthorized_projection() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);

        handle_native_service_error(&ws, Some(ConnectionStatus::Unauthorized));

        assert_eq!(ws.status.get_untracked(), ConnectionStatus::Unauthorized);

        let offline = WsService::new_for_test(ConnectionStatus::Connected);
        handle_native_service_error(&offline, None);
        assert_eq!(
            offline.status.get_untracked(),
            ConnectionStatus::NativeServiceOffline
        );
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

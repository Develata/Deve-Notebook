use super::readiness::{is_current_connection_message, writer_ready_matches};
use super::{ConnectionStatus, WsService};
use crate::api::write_gate::{set_status_and_revoke_writer_ready, status_revokes_writer_ready};
use leptos::prelude::GetUntracked;

#[test]
fn dashboard_metrics_stale_connection_epoch_is_not_current() {
    assert!(is_current_connection_message(3, 3));
    assert!(!is_current_connection_message(2, 3));
}

#[test]
fn writer_ready_requires_matching_repo_and_scope_nonce() {
    assert!(writer_ready_matches(
        Some("repo-a"),
        Some(7),
        Some("repo-a"),
        Some(7),
    ));
    assert!(!writer_ready_matches(
        Some("repo-a"),
        Some(7),
        Some("repo-a"),
        Some(8),
    ));
    assert!(!writer_ready_matches(
        Some("repo-a"),
        Some(7),
        Some("repo-b"),
        Some(7),
    ));
    assert!(!writer_ready_matches(
        Some("repo-a"),
        Some(7),
        Some("repo-a"),
        None,
    ));
}

#[test]
fn native_runtime_readiness_requires_node_role_writer_and_current_scope() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);

    let missing_node_role =
        ws.native_runtime_readiness_for_untracked(Some("repo-a"), Some(7), true);
    assert!(!missing_node_role.node_role_readable);
    assert!(!missing_node_role.is_runtime_ready());

    ws.set_node_role_for_test("main");
    ws.mark_writer_ready("repo-a", 7, "web-light-peer");
    let ready = ws.native_runtime_readiness_for_untracked(Some("repo-a"), Some(7), true);
    assert!(ready.is_runtime_ready());

    ws.set_node_role_probe_failed_for_test();
    let failed_probe = ws.native_runtime_readiness_for_untracked(Some("repo-a"), Some(7), true);
    assert!(!failed_probe.node_role_readable);
    assert!(!failed_probe.is_runtime_ready());
    ws.set_node_role_for_test("main");

    let wrong_repo = ws.native_runtime_readiness_for_untracked(Some("repo-b"), Some(7), true);
    assert!(!wrong_repo.scope_nonce_current);
    assert!(!wrong_repo.writer_ready);
    assert!(!wrong_repo.is_runtime_ready());

    let stale_scope = ws.native_runtime_readiness_for_untracked(Some("repo-a"), Some(8), true);
    assert!(!stale_scope.scope_nonce_current);
    assert!(!stale_scope.writer_ready);
    assert!(!stale_scope.is_runtime_ready());
}

#[test]
fn writer_ready_transport_status_policy_matches_connection_contract() {
    for status in [
        ConnectionStatus::Disconnected,
        ConnectionStatus::Connecting,
        ConnectionStatus::Unauthorized,
        ConnectionStatus::NativeBootstrapInvalid,
        ConnectionStatus::NativeSessionPending,
        ConnectionStatus::NativeServiceOffline,
        ConnectionStatus::NativeReprobeRequired,
    ] {
        assert!(status_revokes_writer_ready(status));
    }
    assert!(!status_revokes_writer_ready(ConnectionStatus::Connected));
}

#[test]
fn writer_ready_is_cleared_on_disconnected_status() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    ws.mark_writer_ready("repo-a", 7, "web-light-peer");

    assert!(set_status_and_revoke_writer_ready(
        ws.set_status,
        ws.writer_ready_reset_signals(),
        ConnectionStatus::Disconnected,
    ));

    assert_eq!(ws.status.get_untracked(), ConnectionStatus::Disconnected);
    assert!(!ws.writer_ready_for(Some("repo-a"), Some(7)));
    assert!(ws.writer_client_id.get_untracked().is_none());
}

#[test]
fn writer_ready_is_cleared_on_unauthorized_status() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    ws.mark_writer_ready("repo-a", 7, "web-light-peer");

    assert!(set_status_and_revoke_writer_ready(
        ws.set_status,
        ws.writer_ready_reset_signals(),
        ConnectionStatus::Unauthorized,
    ));

    assert_eq!(ws.status.get_untracked(), ConnectionStatus::Unauthorized);
    assert!(!ws.writer_ready_for(Some("repo-a"), Some(7)));
    assert!(ws.writer_client_id.get_untracked().is_none());
}

#[test]
fn mark_unauthorized_resets_stale_node_role_runtime_summary() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    ws.complete_foreground_node_role_reprobe("main | git:mirror", "mirror");
    ws.mark_writer_ready("repo-a", 7, "web-light-peer");

    ws.mark_unauthorized();

    assert_eq!(ws.status.get_untracked(), ConnectionStatus::Unauthorized);
    assert_eq!(ws.node_role.get_untracked(), "");
    assert_eq!(ws.source_control_git_bridge.get_untracked(), "unknown");
    assert!(!ws.node_role_probe_failed.get_untracked());
    assert!(!ws.writer_ready_for(Some("repo-a"), Some(7)));
}

#[test]
fn writer_ready_is_cleared_on_native_blocked_status() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    ws.mark_writer_ready("repo-a", 7, "web-light-peer");

    assert!(set_status_and_revoke_writer_ready(
        ws.set_status,
        ws.writer_ready_reset_signals(),
        ConnectionStatus::NativeSessionPending,
    ));

    assert_eq!(
        ws.status.get_untracked(),
        ConnectionStatus::NativeSessionPending
    );
    assert!(!ws.writer_ready_for(Some("repo-a"), Some(7)));
    assert!(ws.writer_client_id.get_untracked().is_none());
}

#[test]
fn connected_status_does_not_clear_existing_writer_ready() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    ws.mark_writer_ready("repo-a", 7, "web-light-peer");

    assert!(set_status_and_revoke_writer_ready(
        ws.set_status,
        ws.writer_ready_reset_signals(),
        ConnectionStatus::Connected,
    ));

    assert!(ws.writer_ready_for(Some("repo-a"), Some(7)));
    assert!(ws.writer_client_id.get_untracked().is_some());
}

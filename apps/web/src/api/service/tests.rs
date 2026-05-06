use super::readiness::{is_current_connection_message, writer_ready_matches};
use super::{ConnectionStatus, WsService};

#[test]
fn dashboard_metrics_stale_connection_epoch_is_not_current() {
    assert!(is_current_connection_message(3, 3));
    assert!(!is_current_connection_message(2, 3));
}

#[test]
fn writer_ready_requires_matching_repo_and_scope_nonce() {
    assert!(writer_ready_matches(
        Some("repo-a".into()),
        Some(7),
        Some("repo-a"),
        Some(7),
    ));
    assert!(!writer_ready_matches(
        Some("repo-a".into()),
        Some(7),
        Some("repo-a"),
        Some(8),
    ));
    assert!(!writer_ready_matches(
        Some("repo-a".into()),
        Some(7),
        Some("repo-b"),
        Some(7),
    ));
    assert!(!writer_ready_matches(
        Some("repo-a".into()),
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
    assert!(!wrong_repo.writer_ready);
    assert!(!wrong_repo.is_runtime_ready());

    let stale_scope = ws.native_runtime_readiness_for_untracked(Some("repo-a"), Some(8), true);
    assert!(!stale_scope.scope_nonce_current);
    assert!(!stale_scope.writer_ready);
    assert!(!stale_scope.is_runtime_ready());
}

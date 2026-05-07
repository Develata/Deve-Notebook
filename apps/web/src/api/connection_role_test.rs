use super::*;
use serde_json::json;

#[test]
fn formats_main_runtime_summary() {
    let summary = format_node_role_summary(&json!({
        "role": "main",
        "ws_port": 3001,
        "main_port": 3001,
        "version": "0.0.1",
        "profile": "standard",
        "delivery": "embedded-frontend",
        "environment": "development",
        "repo_health": {
            "status": "healthy",
            "local_total": 1,
            "healthy": 1,
            "degraded": 0
        }
    }));

    assert_eq!(
        summary,
        "main (ws:3001) | v0.0.1 | standard | embedded-frontend | development | repos:healthy (0/1)"
    );
}

#[test]
fn formats_proxy_runtime_summary() {
    let summary = format_node_role_summary(&json!({
        "role": "proxy",
        "ws_port": 3002,
        "main_port": 3001,
        "version": "0.0.1",
        "profile": "proxy",
        "delivery": "plugin-host-proxy",
        "environment": "production"
    }));

    assert_eq!(
        summary,
        "proxy -> 3001 (ws:3002) | v0.0.1 | proxy | plugin-host-proxy | production | repos:unknown"
    );
}

#[test]
fn formats_degraded_repo_health() {
    let summary = format_node_role_summary(&json!({
        "role": "main",
        "ws_port": 3001,
        "main_port": 3001,
        "version": "0.0.1",
        "profile": "standard",
        "delivery": "embedded-frontend",
        "environment": "development",
        "repo_health": {
            "status": "degraded",
            "local_total": 2,
            "healthy": 1,
            "degraded": 1
        }
    }));

    assert!(summary.contains("repos:degraded (1/2)"));
}

#[test]
fn derives_http_base_from_ws_url() {
    assert_eq!(
        http_base_from_ws_url("ws://127.0.0.1:3001/ws"),
        "http://127.0.0.1:3001"
    );
    assert_eq!(
        http_base_from_ws_url("wss://example.test/ws"),
        "https://example.test"
    );
}

#[test]
fn stale_node_role_probe_results_do_not_mutate_current_connection() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (node_role, set_node_role) = signal("main".to_string());
    let (probe_failed, set_probe_failed) = signal(false);
    let (connection_epoch, set_connection_epoch) = signal(2u64);

    assert!(!apply_node_role_probe_failure(
        set_node_role,
        set_probe_failed,
        connection_epoch,
        1,
    ));
    assert_eq!(node_role.get_untracked(), "main");
    assert!(!probe_failed.get_untracked());

    assert!(apply_node_role_probe_failure(
        set_node_role,
        set_probe_failed,
        connection_epoch,
        2,
    ));
    assert_eq!(node_role.get_untracked(), "");
    assert!(probe_failed.get_untracked());

    set_connection_epoch.set(3);
    assert!(!apply_node_role_probe_success(
        set_node_role,
        set_probe_failed,
        connection_epoch,
        2,
        "proxy".to_string(),
    ));
    assert_eq!(node_role.get_untracked(), "");
    assert!(probe_failed.get_untracked());
}

use super::*;
use serde_json::json;

fn complete_node_role_payload(git_bridge: &str) -> serde_json::Value {
    json!({
        "role": "main",
        "ws_port": 3001,
        "main_port": 3001,
        "version": "0.0.1",
        "profile": "standard",
        "delivery": "embedded-frontend",
        "environment": "development",
        "source_control": {
            "git_bridge": git_bridge
        },
        "repo_health": {
            "status": "healthy",
            "local_total": 1,
            "healthy": 1,
            "degraded": 0
        }
    })
}

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
        "source_control": {
            "git_bridge": "mirror"
        },
        "repo_health": {
            "status": "healthy",
            "local_total": 1,
            "healthy": 1,
            "degraded": 0
        }
    }));

    assert_eq!(
        summary,
        "main (ws:3001) | v0.0.1 | standard | embedded-frontend | development | repos:healthy (0/1) | git:mirror"
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
        "proxy -> 3001 (ws:3002) | v0.0.1 | proxy | plugin-host-proxy | production | repos:unknown | git:unknown"
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
        "source_control": {
            "git_bridge": "off"
        },
        "repo_health": {
            "status": "degraded",
            "local_total": 2,
            "healthy": 1,
            "degraded": 1
        }
    }));

    assert!(summary.contains("repos:degraded (1/2)"));
    assert!(summary.contains("git:off"));
}

#[test]
fn node_role_probe_result_carries_source_control_git_bridge() {
    let result = NodeRoleProbeResult::from_json(&complete_node_role_payload("off"))
        .expect("valid node role payload");

    assert!(result.summary.contains("git:off"));
    assert_eq!(result.source_control_git_bridge, "off");
}

#[test]
fn node_role_probe_result_normalizes_unknown_source_control_git_bridge() {
    let result = NodeRoleProbeResult::from_json(&complete_node_role_payload("native"))
        .expect("valid node role payload");

    assert!(result.summary.contains("git:unknown"));
    assert_eq!(result.source_control_git_bridge, "unknown");
}

#[test]
fn node_role_probe_result_rejects_non_node_role_payload() {
    assert!(NodeRoleProbeResult::from_json(&json!({ "status": "ok" })).is_none());
    assert!(
        NodeRoleProbeResult::from_json(&json!({
            "role": "main",
            "ws_port": 3001
        }))
        .is_none()
    );
}

#[test]
fn node_role_probe_result_rejects_partial_release_runtime_shape() {
    assert!(
        NodeRoleProbeResult::from_json(&json!({
            "role": "main",
            "ws_port": 3001,
            "main_port": 3001,
            "version": "0.0.1",
            "profile": "standard",
            "delivery": "embedded-frontend",
            "environment": "development",
            "source_control": {
                "git_bridge": "mirror"
            }
        }))
        .is_none()
    );
    assert!(
        NodeRoleProbeResult::from_json(&json!({
            "role": "main",
            "ws_port": 3001,
            "main_port": 3001,
            "version": "0.0.1",
            "profile": "standard",
            "delivery": "embedded-frontend",
            "environment": "development",
            "repo_health": {
                "status": "healthy",
                "local_total": 1
            },
            "source_control": {
                "git_bridge": "mirror"
            }
        }))
        .is_none()
    );
    assert!(
        NodeRoleProbeResult::from_json(&json!({
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
                "degraded": 0
            }
        }))
        .is_none()
    );
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
fn ws_url_to_http_base_only_rewrites_leading_scheme_and_ws_suffix() {
    assert_eq!(
        http_base_from_ws_url("ws://127.0.0.1:3001/ws?next=ws://shadow/ws"),
        "http://127.0.0.1:3001"
    );
    assert_eq!(
        http_base_from_ws_url("custom://127.0.0.1:3001/ws"),
        "custom://127.0.0.1:3001"
    );
}

#[test]
fn node_role_probe_url_never_appends_after_query_or_fragment() {
    assert_eq!(
        node_role_url_for_http_base("http://127.0.0.1:3001?next=/ws"),
        "http://127.0.0.1:3001/api/node/role"
    );
    assert_eq!(
        node_role_url_for_http_base("http://127.0.0.1:3001/#/doc"),
        "http://127.0.0.1:3001/api/node/role"
    );
}

#[test]
fn stale_node_role_probe_results_do_not_mutate_current_connection() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let lifecycle = ConnectionLifecycle::new();
    let (node_role, set_node_role) = signal("main".to_string());
    let (source_control_git_bridge, set_source_control_git_bridge) = signal("mirror".to_string());
    let (probe_failed, set_probe_failed) = signal(false);
    let (connection_epoch, set_connection_epoch) = signal(2u64);

    assert!(!apply_node_role_probe_failure(
        &lifecycle,
        set_node_role,
        set_source_control_git_bridge,
        set_probe_failed,
        connection_epoch,
        1,
    ));
    assert_eq!(node_role.get_untracked(), "main");
    assert_eq!(source_control_git_bridge.get_untracked(), "mirror");
    assert!(!probe_failed.get_untracked());

    assert!(apply_node_role_probe_failure(
        &lifecycle,
        set_node_role,
        set_source_control_git_bridge,
        set_probe_failed,
        connection_epoch,
        2,
    ));
    assert_eq!(node_role.get_untracked(), "");
    assert_eq!(source_control_git_bridge.get_untracked(), "unknown");
    assert!(probe_failed.get_untracked());

    set_connection_epoch.set(3);
    assert!(!apply_node_role_probe_success(
        &lifecycle,
        set_node_role,
        set_source_control_git_bridge,
        set_probe_failed,
        connection_epoch,
        2,
        NodeRoleProbeResult {
            summary: "proxy".to_string(),
            source_control_git_bridge: "off".to_string(),
        },
    ));
    assert_eq!(node_role.get_untracked(), "");
    assert_eq!(source_control_git_bridge.get_untracked(), "unknown");
    assert!(probe_failed.get_untracked());

    assert!(apply_node_role_probe_success(
        &lifecycle,
        set_node_role,
        set_source_control_git_bridge,
        set_probe_failed,
        connection_epoch,
        3,
        NodeRoleProbeResult {
            summary: "main".to_string(),
            source_control_git_bridge: "off".to_string(),
        },
    ));
    assert_eq!(node_role.get_untracked(), "main");
    assert_eq!(source_control_git_bridge.get_untracked(), "off");
    assert!(!probe_failed.get_untracked());
}

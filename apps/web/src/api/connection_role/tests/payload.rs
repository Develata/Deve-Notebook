//! plan_ref:
//!   - 18_release#runtime-observability
//!

use super::super::{
    NodeRoleProbeResult, WatcherHealthSnapshot, WatcherHealthStatus, format_node_role_summary,
};
use super::complete_node_role_payload;
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
        "source_control": {
            "authority": "ngit",
            "git_main_mirror": "main"
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
        "main (ws:3001) | v0.0.1 | standard | embedded-frontend | development | repos:healthy (0/1) | sc:ngit/main"
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
        "proxy -> 3001 (ws:3002) | v0.0.1 | proxy | plugin-host-proxy | production | repos:unknown | sc:unknown/unknown"
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
            "authority": "ngit",
            "git_main_mirror": "main"
        },
        "repo_health": {
            "status": "degraded",
            "local_total": 2,
            "healthy": 1,
            "degraded": 1
        }
    }));

    assert!(summary.contains("repos:degraded (1/2)"));
    assert!(summary.contains("sc:ngit/main"));
}

#[test]
fn node_role_probe_result_carries_source_control_authority() {
    let result = NodeRoleProbeResult::from_json(&complete_node_role_payload("ngit"))
        .expect("valid node role payload");

    assert!(result.summary.contains("sc:ngit/main"));
    assert_eq!(result.source_control_authority, "ngit");
    assert!(result.host_file_copy_absolute_path);
    assert!(!result.host_file_reveal_in_system_explorer);
    assert_eq!(
        result.watcher_health,
        WatcherHealthSnapshot {
            status: WatcherHealthStatus::Healthy,
            expected: 1,
            running: 1,
            unavailable: 0,
        }
    );
}

#[test]
fn node_role_probe_preserves_backend_watcher_health_without_recomputing_status() {
    for status in ["healthy", "transitioning", "degraded", "unknown"] {
        let mut payload = complete_node_role_payload("ngit");
        payload["watcher_health"] = json!({
            "status": status,
            "expected": 4,
            "running": 4,
            "unavailable": 0
        });

        let result = NodeRoleProbeResult::from_json(&payload).expect("valid watcher health");

        assert_eq!(result.watcher_health.status.as_str(), status);
        assert_eq!(result.watcher_health.expected, 4);
        assert_eq!(result.watcher_health.running, 4);
        assert_eq!(result.watcher_health.unavailable, 0);
    }
}

#[test]
fn node_role_probe_rejects_invalid_or_partial_watcher_health() {
    let mut invalid_status = complete_node_role_payload("ngit");
    invalid_status["watcher_health"]["status"] = json!("failed");
    assert!(NodeRoleProbeResult::from_json(&invalid_status).is_none());

    let mut partial = complete_node_role_payload("ngit");
    partial["watcher_health"] = json!({
        "status": "healthy",
        "expected": 1,
        "running": 1
    });
    assert!(NodeRoleProbeResult::from_json(&partial).is_none());
}

#[test]
fn node_role_probe_result_accepts_explicit_unknown_source_control_authority() {
    let result = NodeRoleProbeResult::from_json(&complete_node_role_payload("unknown"))
        .expect("valid node role payload");

    assert!(result.summary.contains("sc:unknown/main"));
    assert_eq!(result.source_control_authority, "unknown");
}

#[test]
fn node_role_probe_result_rejects_unknown_source_control_authority_mode() {
    assert!(NodeRoleProbeResult::from_json(&complete_node_role_payload("native")).is_none());
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
                "authority": "ngit",
                "git_main_mirror": "main"
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
            },
            "host_file_actions": {
                "copy_absolute_path": true,
                "reveal_in_system_explorer": true
            },
            "source_control": {
                "authority": "ngit",
                "git_main_mirror": "main"
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
            },
            "host_file_actions": {
                "copy_absolute_path": true,
                "reveal_in_system_explorer": true
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
                "healthy": 1,
                "degraded": 0
            },
            "source_control": {
                "authority": "ngit",
                "git_main_mirror": "main"
            }
        }))
        .is_none()
    );
}

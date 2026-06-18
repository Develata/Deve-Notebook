// apps/cli/src/server/node_role_http.rs
//! plan_ref:
//!   - 07_network#full-peer-mesh-v1
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!   - 18_release#runtime-observability

use axum::Json;
use axum::response::IntoResponse;

use crate::server::node_role;

pub async fn role() -> impl IntoResponse {
    let r = node_role::get_node_role();
    Json(role_payload(&r))
}

fn role_payload(r: &node_role::NodeRole) -> serde_json::Value {
    serde_json::json!({
        "role": r.role,
        "ws_port": r.ws_port,
        "main_port": r.main_port,
        "version": r.version,
        "profile": r.profile,
        "delivery": r.delivery,
        "environment": r.environment,
        "repo_health": {
            "status": r.repo_health.status,
            "local_total": r.repo_health.local_total,
            "healthy": r.repo_health.healthy,
            "degraded": r.repo_health.degraded,
        },
        "source_control": {
            "git_bridge": r.source_control.git_bridge,
        },
        "p2p": p2p_payload(&r.p2p),
        "native_service": r.native_service.as_ref().map(native_service_payload),
    })
}

fn p2p_payload(p2p: &node_role::P2pSummary) -> serde_json::Value {
    serde_json::json!({
        "enabled": p2p.enabled,
        "peers": p2p.peers.iter().map(|peer| serde_json::json!({
            "label": peer.label,
            "peer_id": peer.peer_id,
            "repo_id": peer.repo_id,
            "state": peer.state,
            "attempts": peer.attempts,
            "handshakes": peer.handshakes,
            "sent_pushes": peer.sent_pushes,
            "sent_snapshots": peer.sent_snapshots,
            "applied_pushes": peer.applied_pushes,
            "applied_snapshots": peer.applied_snapshots,
            "last_error_code": peer.last_error_code,
        })).collect::<Vec<_>>(),
    })
}

fn native_service_payload(service: &node_role::NativeServiceSummary) -> serde_json::Value {
    serde_json::json!({
        "state": service.state,
        "endpoint": service.endpoint.as_ref().map(|endpoint| serde_json::json!({
            "http_base": endpoint.http_base,
            "ws_base": endpoint.ws_base,
            "node_role": endpoint.node_role,
            "session_bound": endpoint.session_bound,
        })),
        "offline": service.offline.as_ref().map(|offline| serde_json::json!({
            "retryable": offline.retryable,
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_payload_exposes_runtime_release_shape() {
        let payload = role_payload(&node_role::NodeRole {
            role: "main".into(),
            ws_port: 3001,
            main_port: 3001,
            version: "0.0.1".into(),
            profile: "standard".into(),
            delivery: "embedded-frontend".into(),
            environment: "development".into(),
            repo_health: node_role::RepoHealthSummary::from_degraded_count(2, 1),
            source_control: node_role::SourceControlSummary::from_git_bridge(
                deve_core::config::GitBridgeMode::Mirror,
            ),
            p2p: node_role::P2pSummary::disabled(),
            native_service: None,
        });

        assert_eq!(payload["role"], "main");
        assert_eq!(payload["version"], "0.0.1");
        assert_eq!(payload["profile"], "standard");
        assert_eq!(payload["delivery"], "embedded-frontend");
        assert_eq!(payload["environment"], "development");
        assert_eq!(payload["repo_health"]["status"], "degraded");
        assert_eq!(payload["repo_health"]["local_total"], 2);
        assert_eq!(payload["repo_health"]["healthy"], 1);
        assert_eq!(payload["repo_health"]["degraded"], 1);
        assert_eq!(payload["source_control"]["git_bridge"], "mirror");
        assert_eq!(payload["native_service"], serde_json::Value::Null);
    }

    #[test]
    fn role_payload_exposes_native_service_surface_when_present() {
        let payload = role_payload(&node_role::NodeRole {
            role: "native-main".into(),
            ws_port: 3001,
            main_port: 3001,
            version: "0.0.1".into(),
            profile: "standard".into(),
            delivery: "embedded-frontend".into(),
            environment: "development".into(),
            repo_health: node_role::RepoHealthSummary::unknown(),
            source_control: node_role::SourceControlSummary::from_git_bridge(
                deve_core::config::GitBridgeMode::Mirror,
            ),
            p2p: node_role::P2pSummary::disabled(),
            native_service: Some(node_role::NativeServiceSummary {
                state: "endpoint_ready".into(),
                endpoint: Some(deve_core::native_adapter::NativeEndpointReady {
                    http_base: "http://127.0.0.1:3001".into(),
                    ws_base: "ws://127.0.0.1:3001".into(),
                    node_role: "native-main".into(),
                    session_bound: true,
                }),
                offline: None,
            }),
        });

        assert_eq!(payload["role"], "native-main");
        assert_eq!(payload["native_service"]["state"], "endpoint_ready");
        assert_eq!(
            payload["native_service"]["endpoint"]["http_base"],
            "http://127.0.0.1:3001"
        );
        assert_eq!(payload["native_service"]["endpoint"]["session_bound"], true);
    }

    #[test]
    fn p2p_node_role_summary_omits_token_material() {
        let payload = role_payload(&node_role::NodeRole {
            role: "main".into(),
            ws_port: 3001,
            main_port: 3001,
            version: "0.0.1".into(),
            profile: "standard".into(),
            delivery: "embedded-frontend".into(),
            environment: "development".into(),
            repo_health: node_role::RepoHealthSummary::unknown(),
            source_control: node_role::SourceControlSummary::from_git_bridge(
                deve_core::config::GitBridgeMode::Off,
            ),
            p2p: node_role::P2pSummary {
                enabled: true,
                peers: vec![node_role::P2pPeerSummary {
                    label: "peer-b".into(),
                    peer_id: "bbbbbbbbbbbb".into(),
                    repo_id: "11111111-1111-1111-1111-111111111111".into(),
                    state: "connected".into(),
                    attempts: 2,
                    handshakes: 1,
                    sent_pushes: 1,
                    sent_snapshots: 0,
                    applied_pushes: 1,
                    applied_snapshots: 0,
                    last_error_code: None,
                }],
            },
            native_service: None,
        });

        assert_eq!(payload["p2p"]["enabled"], true);
        assert_eq!(payload["source_control"]["git_bridge"], "off");
        assert_eq!(payload["p2p"]["peers"][0]["label"], "peer-b");
        assert_eq!(payload["p2p"]["peers"][0]["state"], "connected");
        assert!(payload.to_string().find("token").is_none());
        assert!(payload.to_string().find("auth_token_env").is_none());
    }

    #[test]
    fn native_service_payload_omits_internal_offline_reason() {
        let payload = role_payload(&node_role::NodeRole {
            role: "native-main".into(),
            ws_port: 3001,
            main_port: 3001,
            version: "0.0.1".into(),
            profile: "standard".into(),
            delivery: "embedded-frontend".into(),
            environment: "development".into(),
            repo_health: node_role::RepoHealthSummary::unknown(),
            source_control: node_role::SourceControlSummary::from_git_bridge(
                deve_core::config::GitBridgeMode::Mirror,
            ),
            p2p: node_role::P2pSummary::disabled(),
            native_service: Some(node_role::NativeServiceSummary {
                state: "service_offline".into(),
                endpoint: None,
                offline: Some(deve_core::native_adapter::NativeServiceOffline {
                    reason: "C:/Users/user/AppData/Local/deve/service.log".into(),
                    retryable: true,
                }),
            }),
        });

        assert_eq!(payload["native_service"]["state"], "service_offline");
        assert_eq!(payload["native_service"]["offline"]["retryable"], true);
        assert!(payload["native_service"]["offline"].get("reason").is_none());
        assert!(!payload.to_string().contains("AppData"));
    }
}

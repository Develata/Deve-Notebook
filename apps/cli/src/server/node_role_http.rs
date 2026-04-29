// apps/cli/src/server/node_role_http.rs
//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract
//!   - 08_ui_design_03_mobile#mobile-native-adapter-contract
//!   - 15_release#runtime-observability

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
        "native_service": r.native_service.as_ref().map(native_service_payload),
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
            "reason": offline.reason,
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
}

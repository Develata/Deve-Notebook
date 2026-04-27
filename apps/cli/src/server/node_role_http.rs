// apps/cli/src/server/node_role_http.rs

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
        });

        assert_eq!(payload["role"], "main");
        assert_eq!(payload["version"], "0.0.1");
        assert_eq!(payload["profile"], "standard");
        assert_eq!(payload["delivery"], "embedded-frontend");
        assert_eq!(payload["environment"], "development");
    }
}

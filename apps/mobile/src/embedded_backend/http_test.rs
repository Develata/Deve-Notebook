use serde_json::json;

use super::*;

fn plan() -> MobileEmbeddedBackendPlan {
    super::super::plan_mobile_embedded_backend(
        std::env::current_dir()
            .expect("cwd")
            .join("target/mobile-test-data"),
        40123,
    )
    .expect("plan")
}

#[test]
fn mobile_node_role_payload_maps_native_endpoint() {
    let endpoint = endpoint_from_node_role_json(
        &plan(),
        &json!({
            "role": "native-main",
            "native_service": {
                "state": "session_pending",
                "endpoint": {
                    "http_base": "http://127.0.0.1:40123",
                    "ws_base": "ws://127.0.0.1:40123",
                    "node_role": "native-main",
                    "session_bound": false
                }
            }
        }),
    )
    .expect("node role payload");

    assert_eq!(endpoint.http_base, "http://127.0.0.1:40123");
    assert_eq!(endpoint.ws_base, "ws://127.0.0.1:40123");
    assert_eq!(endpoint.node_role, "native-main");
    assert!(!endpoint.session_bound);
}

#[test]
fn mobile_node_role_payload_uses_plan_endpoint_without_native_endpoint() {
    let endpoint = endpoint_from_node_role_json(&plan(), &json!({"role": "native-main"}))
        .expect("minimal node role payload");

    assert_eq!(endpoint.http_base, "http://127.0.0.1:40123");
    assert_eq!(endpoint.ws_base, "ws://127.0.0.1:40123");
    assert_eq!(endpoint.node_role, "native-main");
    assert!(!endpoint.session_bound);
}

#[test]
fn mobile_node_role_payload_requires_top_level_role() {
    let error = endpoint_from_node_role_json(
        &plan(),
        &json!({
            "native_service": {
                "state": "session_pending",
                "endpoint": {
                    "http_base": "http://127.0.0.1:40123",
                    "ws_base": "ws://127.0.0.1:40123",
                    "node_role": "native-main",
                    "session_bound": false
                }
            }
        }),
    )
    .expect_err("missing top-level role fails closed");

    assert!(matches!(
        error,
        MobileEmbeddedBackendError::ProbeInvalidResponse
    ));
}

#[test]
fn mobile_node_role_payload_rejects_empty_fallback_role() {
    let error = endpoint_from_node_role_json(&plan(), &json!({"role": "  "}))
        .expect_err("empty fallback role fails closed");

    assert!(matches!(
        error,
        MobileEmbeddedBackendError::ProbeInvalidResponse
    ));
}

#[test]
fn mobile_node_role_payload_rejects_empty_endpoint_role() {
    for node_role in ["", "  "] {
        let error = endpoint_from_node_role_json(
            &plan(),
            &json!({
                "role": "native-main",
                "native_service": {
                    "state": "session_pending",
                    "endpoint": {
                        "http_base": "http://127.0.0.1:40123",
                        "ws_base": "ws://127.0.0.1:40123",
                        "node_role": node_role,
                        "session_bound": false
                    }
                }
            }),
        )
        .expect_err("empty endpoint role fails closed");

        assert!(matches!(
            error,
            MobileEmbeddedBackendError::ProbeInvalidResponse
        ));
    }
}

#[test]
fn mobile_node_role_payload_rejects_endpoint_from_another_transport_generation() {
    let error = endpoint_from_node_role_json(
        &plan(),
        &json!({
            "role": "native-main",
            "native_service": {
                "endpoint": {
                    "http_base": "http://127.0.0.1:40124",
                    "ws_base": "ws://127.0.0.1:40124",
                    "node_role": "native-main",
                    "session_bound": false
                }
            }
        }),
    )
    .expect_err("foreign generation endpoint fails closed");

    assert!(matches!(
        error,
        MobileEmbeddedBackendError::ProbeInvalidResponse
    ));
}

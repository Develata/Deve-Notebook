use super::{http_backend_capabilities, init_from_config};
use axum::Router;
use axum::body::{self, Body};
use axum::http::{Request, StatusCode};
use axum::routing::get;
use serde_json::Value;
use tokio::sync::Mutex;
use tower::ServiceExt;

static POLICY_TEST_LOCK: Mutex<()> = Mutex::const_new(());

#[tokio::test]
async fn backend_capabilities_http_reports_trusted_cli_fallback_to_native() {
    let _guard = POLICY_TEST_LOCK.lock().await;
    let mut config = deve_core::config::Config::load();
    config.ai.mode = "trusted-cli".to_string();
    config.ai.native_enabled = true;
    config.ai.agent_bridge.enabled = false;
    config.ai.agent_bridge.trusted = false;
    init_from_config(&config);

    let json = get_capabilities_json().await;

    assert_eq!(json["native_available"], true);
    assert_eq!(json["trusted_cli_available"], false);
    assert_eq!(json["effective_backend"], "native");
    assert_eq!(json["effective_backend_reason"], "external agent disabled");
}

#[tokio::test]
async fn backend_capabilities_http_reports_none_when_native_is_disabled() {
    let _guard = POLICY_TEST_LOCK.lock().await;
    let mut config = deve_core::config::Config::load();
    config.ai.mode = "native".to_string();
    config.ai.native_enabled = false;
    config.ai.agent_bridge.enabled = false;
    config.ai.agent_bridge.trusted = false;
    init_from_config(&config);

    let json = get_capabilities_json().await;

    assert_eq!(json["native_available"], false);
    assert_eq!(json["trusted_cli_available"], false);
    assert_eq!(json["effective_backend"], "none");
    assert_eq!(json["native_reason"], "native AI disabled by config");
}

async fn get_capabilities_json() -> Value {
    let app = Router::new().route(
        "/api/ai/backend-capabilities",
        get(http_backend_capabilities),
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/ai/backend-capabilities")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = body::to_bytes(response.into_body(), 4096)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("json")
}

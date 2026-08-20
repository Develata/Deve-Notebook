//! plan_ref:
//!   - 13_i18n#i18n-error-code-catalog
//!   - 15_settings#native-ai-provider-settings
//!   - 08_auth#auth-http-endpoints

use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Request, StatusCode, header};
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceExt;

async fn assert_closed_ai_settings_error(
    response: axum::response::Response,
    expected_status: StatusCode,
    expected_code: &str,
) {
    assert_eq!(response.status(), expected_status);
    let bytes = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body, serde_json::json!({ "code": expected_code }));
    assert!(body.get("error").is_none());
    assert!(body.get("message").is_none());
    assert!(body.get("detail").is_none());
}

fn auth_config() -> Arc<deve_core::security::AuthConfig> {
    Arc::new(
        deve_core::security::AuthConfig::from_material(
            "test_secret_key_at_least_32_bytes_long!",
            "operator",
            deve_core::security::auth::password::hash_password("password").unwrap(),
        )
        .unwrap(),
    )
}

#[tokio::test]
async fn ai_settings_route_requires_auth_and_never_returns_raw_key() {
    let (_dir, state, _) = super::sync_hello_test_support::build_state().expect("state");
    let config = auth_config();
    let app = super::router::build_app(state, 3001, config.clone())
        .expect("router")
        .layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3001))));

    let unauthorized = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/ai/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let token = deve_core::security::auth::jwt::issue_token(
        &config.secret,
        &config.username,
        config.token_version,
    )
    .unwrap();
    let authorized = app
        .oneshot(
            Request::builder()
                .uri("/api/ai/settings")
                .header(header::COOKIE, format!("token={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(authorized.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(authorized.into_body(), 16 * 1024)
        .await
        .unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(body.contains("\"key_configured\""));
    assert!(!body.contains("fixture-secret"));
    assert!(!body.contains("api_key"));
}

#[tokio::test]
async fn ai_settings_route_rejects_oversized_payload_before_deserialization() {
    let (_dir, state, _) = super::sync_hello_test_support::build_state().expect("state");
    let config = auth_config();
    let token = deve_core::security::auth::jwt::issue_token(
        &config.secret,
        &config.username,
        config.token_version,
    )
    .unwrap();
    let app = super::router::build_app(state, 3001, config)
        .expect("router")
        .layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3001))));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/ai/settings")
                .header(header::COOKIE, format!("token={token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(vec![b'x'; 16 * 1024 + 1]))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_closed_ai_settings_error(
        response,
        StatusCode::BAD_REQUEST,
        "AI_SETTINGS_INVALID",
    )
    .await;
}

#[tokio::test]
async fn native_ai_settings_put_cors_preflight_requires_exact_origin() {
    let (_dir, state, _) = super::sync_hello_test_support::build_state().expect("state");
    let config = auth_config();
    let origin = "http://tauri.localhost";
    let app = super::router::build_app_with_native_session_and_p2p(
        state,
        3001,
        config,
        None,
        deve_core::config::RuntimeEnvironment::Production,
        Some(&[origin.to_string()]),
        super::router::WsTransportRouterParts::new(
            None,
            super::ws::transport::WsTransportRuntime::new(),
        ),
    )
    .expect("router")
    .layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3001))));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/ai/settings")
                .header(header::ORIGIN, origin)
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "PUT")
                .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|value| value.to_str().ok()),
        Some(origin)
    );
    assert!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_METHODS)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|methods| methods.split(',').any(|method| method.trim() == "PUT"))
    );

    let evil = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/ai/settings")
                .header(header::ORIGIN, "http://tauri.localhost.evil.example")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "PUT")
                .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(
        evil.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|value| value.to_str().ok()),
        Some("http://tauri.localhost.evil.example")
    );
}

#[tokio::test]
async fn ai_settings_put_rejects_unknown_fields() {
    let (_dir, state, _) = super::sync_hello_test_support::build_state().expect("state");
    let config = auth_config();
    let token = deve_core::security::auth::jwt::issue_token(
        &config.secret,
        &config.username,
        config.token_version,
    )
    .unwrap();
    let app = super::router::build_app(state, 3001, config)
        .expect("router")
        .layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3001))));
    let cases = [
        (
            Some("application/json"),
            r#"{"expected_revision":1,"provider":"openai-chat-completions","base_url":"https://api.openai.com/v1","model":"gpt-4o-mini","max_tokens":4096,"unexpected":"secret=/private/path"}"#,
        ),
        (Some("application/json"), r#"{"expected_revision":"#),
        (
            None,
            r#"{"expected_revision":1,"provider":"openai-chat-completions","base_url":"https://api.openai.com/v1","model":"gpt-4o-mini","max_tokens":4096}"#,
        ),
    ];
    for (content_type, body) in cases {
        let mut request = Request::builder()
            .method("PUT")
            .uri("/api/ai/settings")
            .header(header::COOKIE, format!("token={token}"));
        if let Some(content_type) = content_type {
            request = request.header(header::CONTENT_TYPE, content_type);
        }
        let response = app
            .clone()
            .oneshot(
                request
                    .body(Body::from(body))
                    .expect("AI settings rejection request"),
            )
            .await
            .unwrap();
        assert_closed_ai_settings_error(
            response,
            StatusCode::BAD_REQUEST,
            "AI_SETTINGS_INVALID",
        )
        .await;
    }
}

#[tokio::test]
async fn ai_settings_http_error_body_is_closed_typed_code() {
    let (_dir, state, _) = super::sync_hello_test_support::build_state().expect("state");
    let config = auth_config();
    let token = deve_core::security::auth::jwt::issue_token(
        &config.secret,
        &config.username,
        config.token_version,
    )
    .unwrap();
    let app = super::router::build_app(state, 3001, config)
        .expect("router")
        .layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3001))));
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/ai/settings")
                .header(header::COOKIE, format!("token={token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"expected_revision":0,"provider":"openai-chat-completions","base_url":"https://api.openai.com/v1","model":"gpt-4o-mini","max_tokens":4096,"clear_api_key":false}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_closed_ai_settings_error(
        response,
        StatusCode::CONFLICT,
        "AI_SETTINGS_REVISION_CONFLICT",
    )
    .await;
}

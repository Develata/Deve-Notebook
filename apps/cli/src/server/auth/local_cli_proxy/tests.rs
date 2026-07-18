//! plan_ref:
//!   - 08_auth#local-cli-proxy-authority
//!
//! Local CLI proxy admission and replay tests.

use super::*;
use crate::local_cli_proxy_contract::LocalCliRemoteImportRequest;
use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{HeaderValue, Request, StatusCode, header};
use deve_core::protocol::{RemoteImportRequest, RemoteImportRequestContext, ScopeNonce};
use std::sync::Arc;
use tower::ServiceExt;

fn config() -> AuthConfig {
    AuthConfig::from_material(
        "test_secret_key_at_least_32_bytes_long!",
        "operator",
        deve_core::security::auth::password::hash_password("password").unwrap(),
    )
    .unwrap()
}

fn request(request_id: Uuid, repo_id: RepoId, scope_nonce: u64) -> Vec<u8> {
    serde_json::to_vec(&LocalCliRemoteImportRequest::Intent {
        request: RemoteImportRequest::List {
            context: RemoteImportRequestContext {
                request_id,
                repo_id,
                branch: None,
                scope_nonce: ScopeNonce::new(scope_nonce),
            },
        },
    })
    .unwrap()
}

fn bearer_headers(config: &AuthConfig) -> HeaderMap {
    let token = jwt::issue_token(&config.secret, &config.username, config.token_version).unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    headers
}

#[test]
fn loopback_bearer_admits_exact_replay_and_rejects_digest_conflict() {
    let config = config();
    let headers = bearer_headers(&config);
    let gateway = LocalCliProxyGateway::default();
    let request_id = Uuid::new_v4();
    let repo_id = Uuid::new_v4();
    let body = request(request_id, repo_id, 1);
    let peer = SocketAddr::from(([127, 0, 0, 1], 32001));

    let (authority, _) = gateway.admit(peer, &headers, &config, &body).unwrap();
    assert_eq!(authority.request_id(), request_id);
    assert_eq!(authority.repo_id(), repo_id);
    assert_eq!(authority.operation(), "list");
    gateway.admit(peer, &headers, &config, &body).unwrap();

    let conflict = request(request_id, repo_id, 2);
    assert_eq!(
        gateway
            .admit(peer, &headers, &config, &conflict)
            .unwrap_err(),
        forbidden()
    );
}

#[test]
fn admission_rejects_non_loopback_cookie_and_zero_scope() {
    let config = config();
    let headers = bearer_headers(&config);
    let gateway = LocalCliProxyGateway::default();
    let body = request(Uuid::new_v4(), Uuid::new_v4(), 1);
    assert_eq!(
        gateway
            .admit(
                SocketAddr::from(([192, 0, 2, 1], 32001)),
                &headers,
                &config,
                &body,
            )
            .unwrap_err(),
        forbidden()
    );

    let mut cookie_headers = headers.clone();
    cookie_headers.insert(header::COOKIE, HeaderValue::from_static("token=browser"));
    assert_eq!(
        gateway
            .admit(
                SocketAddr::from(([127, 0, 0, 1], 32001)),
                &cookie_headers,
                &config,
                &body,
            )
            .unwrap_err(),
        forbidden()
    );

    let zero_scope = request(Uuid::new_v4(), Uuid::new_v4(), 0);
    assert_eq!(
        gateway
            .admit(
                SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], 32001)),
                &headers,
                &config,
                &zero_scope,
            )
            .unwrap_err(),
        forbidden()
    );
}

#[test]
fn admission_rejects_cookie_only_and_missing_bearer() {
    let config = config();
    let gateway = LocalCliProxyGateway::default();
    let body = request(Uuid::new_v4(), Uuid::new_v4(), 1);
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, HeaderValue::from_static("token=browser"));
    assert_eq!(
        gateway
            .admit(
                SocketAddr::from(([127, 0, 0, 1], 32001)),
                &headers,
                &config,
                &body,
            )
            .unwrap_err(),
        forbidden()
    );
    assert_eq!(
        gateway
            .admit(
                SocketAddr::from(([127, 0, 0, 1], 32001)),
                &HeaderMap::new(),
                &config,
                &body,
            )
            .unwrap_err(),
        missing_token()
    );
}

#[test]
fn admission_keeps_oversized_body_rejection_as_defense_in_depth() {
    let config = config();
    let headers = bearer_headers(&config);
    let gateway = LocalCliProxyGateway::default();
    let peer = SocketAddr::from(([127, 0, 0, 1], 32001));
    let mut body = request(Uuid::new_v4(), Uuid::new_v4(), 1);
    body.resize(LOCAL_CLI_PROXY_MAX_REQUEST_BODY_BYTES + 1, b' ');

    assert_eq!(
        gateway.admit(peer, &headers, &config, &body).unwrap_err(),
        forbidden()
    );
}

#[tokio::test]
async fn http_route_rejects_oversized_body_before_gateway_admission() {
    let (_dir, state, _) = crate::server::sync_hello_test_support::build_state().expect("state");
    let config = Arc::new(config());
    let app = crate::server::router::build_app(state, 3001, config.clone())
        .expect("router")
        .layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3001))));
    let headers = bearer_headers(&config);
    let mut request = Request::builder()
        .method("POST")
        .uri("/api/local-cli/remote-import")
        .body(Body::from(vec![
            b'x';
            LOCAL_CLI_PROXY_MAX_REQUEST_BODY_BYTES + 1
        ]))
        .expect("request");
    request.headers_mut().extend(headers);

    let response = app.oneshot(request).await.expect("response");

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn http_route_allows_valid_body_within_limit() {
    let (_dir, state, repo_id) =
        crate::server::sync_hello_test_support::build_state().expect("state");
    let config = Arc::new(config());
    let app = crate::server::router::build_app(state, 3001, config.clone())
        .expect("router")
        .layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3001))));
    let headers = bearer_headers(&config);
    let mut body = request(Uuid::new_v4(), repo_id, 1);
    body.resize(LOCAL_CLI_PROXY_MAX_REQUEST_BODY_BYTES, b' ');
    assert_eq!(body.len(), LOCAL_CLI_PROXY_MAX_REQUEST_BODY_BYTES);
    let mut request = Request::builder()
        .method("POST")
        .uri("/api/local-cli/remote-import")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .expect("request");
    request.headers_mut().extend(headers);

    let response = app.oneshot(request).await.expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}

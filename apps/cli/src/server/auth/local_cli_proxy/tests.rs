//! plan_ref:
//!   - 08_auth#local-cli-proxy-authority
//!
//! Local CLI proxy admission and replay tests.

use super::*;
use crate::local_cli_proxy_contract::{
    LocalCliRemoteImportRequest, LocalCliRepoRemovalRequest, LocalCliRepoRemovalResponse,
};
use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{HeaderValue, Request, StatusCode, header};
use deve_core::protocol::{
    RemoteImportRequest, RemoteImportRequestContext, RepoLifecycleOperation, RepoLifecycleState,
    ScopeNonce, SwitchNonce,
};
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

fn removal_prepare(request_id: Uuid, repo_id: RepoId) -> Vec<u8> {
    serde_json::to_vec(&LocalCliRepoRemovalRequest::Prepare {
        request_id,
        repo_id,
        current_scope_nonce: ScopeNonce::new(1),
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
fn removal_principal_is_stable_across_login_sessions_but_replay_is_session_scoped() {
    let config = config();
    let first_headers = bearer_headers(&config);
    let second_headers = bearer_headers(&config);
    let gateway = LocalCliProxyGateway::default();
    let body = removal_prepare(Uuid::new_v4(), Uuid::new_v4());
    let peer = SocketAddr::from(([127, 0, 0, 1], 32001));

    let (first, _) = gateway
        .admit_repo_removal(peer, &first_headers, &config, &body)
        .unwrap();
    let (second, _) = gateway
        .admit_repo_removal(peer, &second_headers, &config, &body)
        .unwrap();

    assert_eq!(first.principal_digest(), second.principal_digest());
    assert_eq!(first.operation(), "prepare");
}

#[test]
fn removal_execute_rejects_zero_scope_stale_switch_and_nil_preparation() {
    let config = config();
    let headers = bearer_headers(&config);
    let gateway = LocalCliProxyGateway::default();
    let peer = SocketAddr::from(([127, 0, 0, 1], 32001));
    let token =
        deve_core::protocol::RemovalConfirmationToken::from_backend("a".repeat(64)).expect("token");
    let body = serde_json::to_vec(&LocalCliRepoRemovalRequest::Execute {
        request_id: Uuid::new_v4(),
        repo_id: Uuid::new_v4(),
        preparation_id: Uuid::nil(),
        confirmation_token: token,
        fallback_binding: None,
        current_scope_nonce: ScopeNonce::new(1),
        switch_nonce: deve_core::protocol::SwitchNonce::new(1),
    })
    .unwrap();

    assert_eq!(
        gateway
            .admit_repo_removal(peer, &headers, &config, &body)
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

#[tokio::test]
async fn repo_removal_route_returns_typed_preview_without_exposing_paths() {
    let (_dir, state, repo_id) =
        crate::server::sync_hello_test_support::build_state().expect("state");
    let config = Arc::new(config());
    let app = crate::server::router::build_app(state, 3001, config.clone())
        .expect("router")
        .layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3001))));
    let headers = bearer_headers(&config);
    let body = removal_prepare(Uuid::new_v4(), repo_id);
    let mut request = Request::builder()
        .method("POST")
        .uri("/api/local-cli/repo-removal")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .expect("request");
    request.headers_mut().extend(headers);

    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body");
    let response: LocalCliRepoRemovalResponse =
        serde_json::from_slice(&bytes).expect("typed response");
    let LocalCliRepoRemovalResponse::Prepared {
        repo_id: actual_repo_id,
        preview,
        ..
    } = response
    else {
        panic!("expected prepared response")
    };
    assert_eq!(actual_repo_id, repo_id);
    assert!(!preview.deleted.is_empty());
    let json = String::from_utf8(bytes.to_vec()).expect("utf8");
    assert!(!json.contains("\\\\"));
    assert!(!json.contains("ledger/local"));
}

#[tokio::test]
async fn repo_removal_route_executes_and_status_is_remove_only() {
    let (_dir, state, repo_id) =
        crate::server::sync_hello_test_support::build_state().expect("state");
    let config = Arc::new(config());
    let app = crate::server::router::build_app(state, 3001, config.clone())
        .expect("router")
        .layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3001))));
    let headers = bearer_headers(&config);

    let prepare_request_id = Uuid::new_v4();
    let prepared = send_removal(&app, &headers, removal_prepare(prepare_request_id, repo_id)).await;
    let LocalCliRepoRemovalResponse::Prepared {
        preparation_id,
        confirmation_token: Some(confirmation_token),
        ..
    } = prepared
    else {
        panic!("expected executable removal preview")
    };

    let execute_request_id = Uuid::new_v4();
    let execute = serde_json::to_vec(&LocalCliRepoRemovalRequest::Execute {
        request_id: execute_request_id,
        repo_id,
        preparation_id,
        confirmation_token,
        fallback_binding: None,
        current_scope_nonce: ScopeNonce::new(1),
        switch_nonce: SwitchNonce::new(2),
    })
    .expect("encode execute");
    let accepted = send_removal(&app, &headers, execute).await;
    assert!(matches!(
        accepted,
        LocalCliRepoRemovalResponse::Accepted {
            request_id,
            repo_id: actual_repo_id,
            ..
        } if request_id == execute_request_id && actual_repo_id == repo_id
    ));

    let status_request_id = Uuid::new_v4();
    for _ in 0..100 {
        let status = serde_json::to_vec(&LocalCliRepoRemovalRequest::Status {
            request_id: status_request_id,
            execute_request_id,
            repo_id,
        })
        .expect("encode status");
        match send_removal(&app, &headers, status).await {
            LocalCliRepoRemovalResponse::Status {
                request_id,
                execute_request_id: actual_execute_request_id,
                operation,
                state,
                ..
            } => {
                assert_eq!(request_id, status_request_id);
                assert_eq!(actual_execute_request_id, execute_request_id);
                assert_eq!(operation, RepoLifecycleOperation::Remove);
                if state == RepoLifecycleState::Terminal {
                    return;
                }
            }
            other => panic!("unexpected removal status response: {other:?}"),
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("removal lifecycle did not reach terminal state");
}

async fn send_removal(
    app: &axum::Router,
    headers: &HeaderMap,
    body: Vec<u8>,
) -> LocalCliRepoRemovalResponse {
    let mut request = Request::builder()
        .method("POST")
        .uri("/api/local-cli/repo-removal")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .expect("request");
    request.headers_mut().extend(headers.clone());
    let response = app.clone().oneshot(request).await.expect("response");
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("typed removal response")
}

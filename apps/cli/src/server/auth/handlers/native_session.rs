//! plan_ref:
//!   - 08_auth#jwt-cookie-contract
//!   - 11_ui_design/02_desktop#desktop-service-supervisor-contract

use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::{
    Extension, Json,
    extract::ConnectInfo,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use deve_core::native_adapter::{
    NATIVE_SESSION_BOOTSTRAP_HEADER, NATIVE_SESSION_BOOTSTRAP_SECRET_ENV,
};
use deve_core::protocol::auth::{AuthErrorCode, LoginResponse};
use deve_core::security::auth::{config::AuthConfig, jwt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeSessionBridgeError {
    MissingSecret,
    SecretTooShort,
}

impl fmt::Display for NativeSessionBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSecret => write!(f, "native session bridge secret is missing"),
            Self::SecretTooShort => write!(f, "native session bridge secret is too short"),
        }
    }
}

impl std::error::Error for NativeSessionBridgeError {}

pub struct NativeSessionBridge {
    secret: String,
    consumed: AtomicBool,
}

impl fmt::Debug for NativeSessionBridge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeSessionBridge")
            .field("secret", &"<redacted>")
            .field("consumed", &self.consumed.load(Ordering::Relaxed))
            .finish()
    }
}

impl NativeSessionBridge {
    pub fn from_env(native_loopback: bool) -> Result<Option<Self>, NativeSessionBridgeError> {
        if !native_loopback {
            return Ok(None);
        }
        let secret = std::env::var(NATIVE_SESSION_BOOTSTRAP_SECRET_ENV)
            .map_err(|_| NativeSessionBridgeError::MissingSecret)?;
        Ok(Some(Self::new(secret)?))
    }

    pub fn new(secret: String) -> Result<Self, NativeSessionBridgeError> {
        if secret.trim().is_empty() {
            return Err(NativeSessionBridgeError::MissingSecret);
        }
        if secret.len() < 32 {
            return Err(NativeSessionBridgeError::SecretTooShort);
        }
        Ok(Self {
            secret,
            consumed: AtomicBool::new(false),
        })
    }

    fn issue_once(&self, presented: &str) -> bool {
        if !constant_time_eq(self.secret.as_bytes(), presented.as_bytes()) {
            return false;
        }
        self.consumed
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }
}

pub async fn native_session(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(config): Extension<Arc<AuthConfig>>,
    Extension(bridge): Extension<Arc<NativeSessionBridge>>,
) -> impl IntoResponse {
    if !addr.ip().is_loopback() {
        return reject();
    }
    let Some(secret) = headers
        .get(NATIVE_SESSION_BOOTSTRAP_HEADER)
        .and_then(|value| value.to_str().ok())
    else {
        return reject();
    };
    if !bridge.issue_once(secret) {
        return reject();
    }

    match jwt::issue_token(&config.secret, &config.username, config.token_version) {
        Ok(token) => {
            crate::server::node_role::mark_native_session_bound();
            (
                StatusCode::OK,
                build_native_loopback_cookie(&token),
                Json(LoginResponse::success()),
            )
        }
        Err(err) => {
            tracing::error!("Native session JWT issue failed: {:?}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                build_empty_cookie(),
                Json(LoginResponse::failure(AuthErrorCode::InternalError)),
            )
        }
    }
}

fn reject() -> (StatusCode, [(String, String); 1], Json<LoginResponse>) {
    (
        StatusCode::UNAUTHORIZED,
        build_empty_cookie(),
        Json(LoginResponse::failure(AuthErrorCode::TokenMissing)),
    )
}

fn build_native_loopback_cookie(token: &str) -> [(String, String); 1] {
    let cookie = Cookie::build(("token", token.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::None)
        .secure(true)
        .build();
    [("Set-Cookie".into(), cookie.to_string())]
}

fn build_empty_cookie() -> [(String, String); 1] {
    [("X-No-Op".into(), "1".into())]
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let max_len = a.len().max(b.len());
    let mut diff = a.len() ^ b.len();
    for idx in 0..max_len {
        let left = a.get(idx).copied().unwrap_or(0);
        let right = b.get(idx).copied().unwrap_or(0);
        diff |= usize::from(left ^ right);
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::{self, Body};
    use axum::extract::connect_info::MockConnectInfo;
    use axum::http::{Request, header};
    use axum::routing::post;
    use tower::ServiceExt;

    #[test]
    fn native_session_bridge_consumes_secret_once() {
        let bridge = NativeSessionBridge::new("a".repeat(64)).expect("bridge");

        assert!(bridge.issue_once(&"a".repeat(64)));
        assert!(!bridge.issue_once(&"a".repeat(64)));
        assert!(!bridge.issue_once(&"b".repeat(64)));
    }

    #[test]
    fn native_session_bridge_rejects_short_secret() {
        assert!(matches!(
            NativeSessionBridge::new("short".into()),
            Err(NativeSessionBridgeError::SecretTooShort)
        ));
    }

    #[test]
    fn native_session_bridge_debug_redacts_secret() {
        let secret = "a".repeat(64);
        let bridge = NativeSessionBridge::new(secret.clone()).expect("bridge");

        assert!(!format!("{bridge:?}").contains(&secret));
    }

    #[test]
    fn native_loopback_cookie_is_http_only_secure_and_cross_site() {
        let cookie = build_native_loopback_cookie("abc");
        let value = &cookie[0].1;

        assert!(value.contains("HttpOnly"));
        assert!(value.contains("SameSite=None"));
        assert!(value.contains("Secure"));
    }

    #[tokio::test]
    async fn native_session_http_issues_loopback_cookie_once() {
        let secret = "a".repeat(64);
        let app = native_session_test_router(&secret, SocketAddr::from(([127, 0, 0, 1], 3001)));

        let response = app
            .clone()
            .oneshot(native_session_request(&secret))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let set_cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|value| value.to_str().ok())
            .expect("set-cookie");
        assert!(set_cookie.starts_with("token="));
        assert!(set_cookie.contains("HttpOnly"));
        assert!(set_cookie.contains("SameSite=None"));
        assert!(set_cookie.contains("Secure"));
        let bytes = body::to_bytes(response.into_body(), 4096)
            .await
            .expect("body");
        let json: LoginResponse = serde_json::from_slice(&bytes).expect("login response");
        assert!(json.success);

        let second = app
            .oneshot(native_session_request(&secret))
            .await
            .expect("second response");
        assert_eq!(second.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn native_session_http_rejects_non_loopback_peer() {
        let secret = "a".repeat(64);
        let app = native_session_test_router(&secret, SocketAddr::from(([203, 0, 113, 1], 3001)));

        let response = app
            .oneshot(native_session_request(&secret))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn default_server_router_does_not_mount_native_session_endpoint() {
        let (_dir, state, _) =
            crate::server::sync_hello_test_support::build_state().expect("state");
        let app = crate::server::router::build_app(
            state,
            3001,
            Arc::new(AuthConfig::dev_default().expect("auth config")),
        )
        .expect("router")
        .layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3001))));

        let response = app
            .oneshot(native_session_request(&"a".repeat(64)))
            .await
            .expect("response");

        assert_ne!(response.status(), StatusCode::OK);
        assert!(response.headers().get(header::SET_COOKIE).is_none());
    }

    fn native_session_test_router(secret: &str, addr: SocketAddr) -> Router {
        Router::new()
            .route("/api/auth/native-session", post(native_session))
            .layer(Extension(Arc::new(
                NativeSessionBridge::new(secret.to_string()).expect("bridge"),
            )))
            .layer(Extension(Arc::new(
                AuthConfig::dev_default().expect("auth config"),
            )))
            .layer(MockConnectInfo(addr))
    }

    fn native_session_request(secret: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/api/auth/native-session")
            .header(NATIVE_SESSION_BOOTSTRAP_HEADER, secret)
            .body(Body::empty())
            .expect("request")
    }
}

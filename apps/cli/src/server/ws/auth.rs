//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! WebSocket browser-session admission policy.

use axum::http::request::Parts;
use deve_core::protocol::auth::AuthErrorCode;
use deve_core::security::auth::{config::AuthConfig, jwt};

pub(super) fn browser_session_admission(
    config: &AuthConfig,
    req: &Parts,
) -> Result<(), AuthErrorCode> {
    let token = cookie_token(req);
    let authed = token.as_deref().is_some_and(|token| {
        jwt::validate_token(&config.secret, token, config.token_version).is_ok()
    });
    if is_browser_session_connection(
        authed,
        config.allow_anonymous_localhost,
        is_local_request(req),
    ) {
        return Ok(());
    }
    Err(token
        .map(|_| AuthErrorCode::TokenExpired)
        .unwrap_or(AuthErrorCode::TokenMissing))
}

pub(super) fn is_browser_session_connection(
    authed: bool,
    allow_anonymous_localhost: bool,
    is_local: bool,
) -> bool {
    authed || (allow_anonymous_localhost && is_local)
}

fn cookie_token(req: &Parts) -> Option<String> {
    req.headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(crate::server::auth::cookie::extract_token_from_cookie_header)
}

fn is_local_request(req: &Parts) -> bool {
    req.extensions
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .is_some_and(|ci| ci.0.ip().is_loopback())
}

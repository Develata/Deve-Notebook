//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! WebSocket browser-session admission policy.

use axum::http::request::Parts;
use deve_core::security::auth::{config::AuthConfig, jwt};

pub(super) fn is_browser_session_request(config: &AuthConfig, req: &Parts) -> bool {
    let authed = cookie_token(req).as_deref().is_some_and(|token| {
        jwt::validate_token(&config.secret, token, config.token_version).is_ok()
    });
    is_browser_session_connection(
        authed,
        config.allow_anonymous_localhost,
        is_local_request(req),
    )
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

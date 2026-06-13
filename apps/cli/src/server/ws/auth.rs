//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! WebSocket Browser / FullPeer admission policy.

use crate::server::source_control_grants::AuthSessionId;
use axum::http::request::Parts;
use deve_core::protocol::auth::AuthErrorCode;
use deve_core::security::auth::config::AuthConfig;

pub(super) const P2P_INBOUND_TOKEN_ENV: &str = "DEVE_P2P_INBOUND_TOKEN";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum WsAdmission {
    Browser(BrowserAdmission),
    FullPeer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BrowserAdmission {
    auth_session_id: AuthSessionId,
    set_cookie: Option<String>,
}

impl WsAdmission {
    #[cfg(test)]
    pub(super) fn is_browser(&self) -> bool {
        matches!(self, Self::Browser(_))
    }

    pub(super) fn browser_auth_session(&self) -> Option<&AuthSessionId> {
        match self {
            Self::Browser(admission) => Some(&admission.auth_session_id),
            Self::FullPeer => None,
        }
    }

    pub(super) fn set_cookie(&self) -> Option<&str> {
        match self {
            Self::Browser(admission) => admission.set_cookie.as_deref(),
            Self::FullPeer => None,
        }
    }
}

pub(super) fn session_admission(
    config: &AuthConfig,
    req: &Parts,
    p2p_inbound_token_env: Option<&str>,
) -> Result<WsAdmission, AuthErrorCode> {
    if has_authorization_header(req) {
        full_peer_admission(req, p2p_inbound_token_env)?;
        return Ok(WsAdmission::FullPeer);
    }

    let cookie_header = req
        .headers
        .get("cookie")
        .and_then(|value| value.to_str().ok());
    let session = crate::server::auth::browser_session::resolve_required(
        config,
        cookie_header,
        is_local_request(req),
    )?;
    Ok(WsAdmission::Browser(BrowserAdmission {
        auth_session_id: session.auth_session_id,
        set_cookie: session.set_cookie,
    }))
}

#[cfg(test)]
pub(super) fn is_browser_session_connection(
    authed: bool,
    allow_anonymous_localhost: bool,
    is_local: bool,
) -> bool {
    authed || (allow_anonymous_localhost && is_local)
}

fn has_authorization_header(req: &Parts) -> bool {
    req.headers.get("authorization").is_some()
}

fn full_peer_admission(
    req: &Parts,
    p2p_inbound_token_env: Option<&str>,
) -> Result<(), AuthErrorCode> {
    let token = bearer_token(req).ok_or(AuthErrorCode::TokenMissing)?;
    let token_env = p2p_inbound_token_env.ok_or(AuthErrorCode::TokenMissing)?;
    let expected = std::env::var(token_env).map_err(|_| AuthErrorCode::TokenMissing)?;
    if expected.is_empty() {
        return Err(AuthErrorCode::TokenMissing);
    }
    if token == expected {
        Ok(())
    } else {
        Err(AuthErrorCode::TokenExpired)
    }
}

fn bearer_token(req: &Parts) -> Option<&str> {
    req.headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

fn is_local_request(req: &Parts) -> bool {
    req.extensions
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .is_some_and(|ci| ci.0.ip().is_loopback())
}

#[cfg(test)]
mod tests;

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
mod tests {
    use super::{P2P_INBOUND_TOKEN_ENV, WsAdmission, session_admission};
    use crate::server::auth::dev_session;
    use crate::server::source_control_grants::AuthSessionId;
    use axum::http::Request;
    use deve_core::security::auth::config::AuthConfig;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn auth_config() -> AuthConfig {
        AuthConfig {
            secret: "test_secret_key_at_least_32_bytes_long!".to_string(),
            username: "admin".to_string(),
            password_hash: "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$ZgAAAA".to_string(),
            allow_anonymous_localhost: false,
            token_version: 1,
        }
    }

    fn parts_with_auth(value: Option<&str>) -> axum::http::request::Parts {
        let mut builder = Request::builder().uri("/ws");
        if let Some(value) = value {
            builder = builder.header("authorization", value);
        }
        let request = builder.body(()).expect("request");
        request.into_parts().0
    }

    fn local_parts_with_cookie(cookie: Option<String>) -> axum::http::request::Parts {
        let mut builder = Request::builder().uri("/ws");
        if let Some(cookie) = cookie {
            builder = builder.header("cookie", cookie);
        }
        let request = builder.body(()).expect("request");
        let (mut parts, _) = request.into_parts();
        parts
            .extensions
            .insert(axum::extract::ConnectInfo(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                3001,
            )));
        parts
    }

    #[test]
    fn bearer_token_admits_full_peer_without_browser_session() {
        let _lock = ENV_LOCK.lock().expect("env lock");
        let _guard = EnvGuard::set(P2P_INBOUND_TOKEN_ENV, Some("mesh-token"));
        let admission = session_admission(
            &auth_config(),
            &parts_with_auth(Some("Bearer mesh-token")),
            Some(P2P_INBOUND_TOKEN_ENV),
        )
        .expect("admission");

        assert_eq!(admission, WsAdmission::FullPeer);
        assert!(!admission.is_browser());
    }

    #[test]
    fn invalid_bearer_token_rejects_full_peer() {
        let _lock = ENV_LOCK.lock().expect("env lock");
        let _guard = EnvGuard::set(P2P_INBOUND_TOKEN_ENV, Some("mesh-token"));

        assert!(
            session_admission(
                &auth_config(),
                &parts_with_auth(Some("Bearer wrong")),
                Some(P2P_INBOUND_TOKEN_ENV),
            )
            .is_err()
        );
    }

    #[test]
    fn bearer_token_uses_configured_inbound_token_env() {
        const CUSTOM_TOKEN_ENV: &str = "DEVE_TEST_P2P_CUSTOM_TOKEN";
        let _lock = ENV_LOCK.lock().expect("env lock");
        let _guard = EnvGuard::set(CUSTOM_TOKEN_ENV, Some("custom-token"));

        let admission = session_admission(
            &auth_config(),
            &parts_with_auth(Some("Bearer custom-token")),
            Some(CUSTOM_TOKEN_ENV),
        )
        .expect("admission");

        assert_eq!(admission, WsAdmission::FullPeer);
    }

    #[test]
    fn anonymous_localhost_ws_uses_dev_session_cookie() {
        let mut config = auth_config();
        config.allow_anonymous_localhost = true;
        let cookie = dev_session::cookie_header_for_test(&config.secret, "ws-browser-session");
        let admission = session_admission(&config, &local_parts_with_cookie(Some(cookie)), None)
            .expect("anonymous localhost admission");
        let expected = AuthSessionId::from_dev_session_cookie(
            &config.username,
            config.token_version,
            "ws-browser-session",
        );

        assert_eq!(admission.browser_auth_session(), Some(&expected));
        assert!(admission.set_cookie().is_none());

        let admission = session_admission(&config, &local_parts_with_cookie(None), None)
            .expect("anonymous localhost admission");
        assert!(admission.browser_auth_session().is_some());
        assert!(admission.set_cookie().is_some());
    }

    struct EnvGuard {
        key: &'static str,
        old: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: Option<&str>) -> Self {
            let old = std::env::var(key).ok();
            // SAFETY: tests serialize mutation through ENV_LOCK and restore the key.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
            Self { key, old }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: EnvGuard owns restoration for this key while ENV_LOCK is held.
            unsafe {
                match &self.old {
                    Some(value) => std::env::set_var(self.key, value),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }
}

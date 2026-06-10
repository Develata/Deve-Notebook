//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! WebSocket Browser / FullPeer admission policy.

use crate::server::source_control_grants::AuthSessionId;
use axum::http::request::Parts;
use deve_core::protocol::auth::AuthErrorCode;
use deve_core::security::auth::{config::AuthConfig, jwt};

pub(super) const P2P_INBOUND_TOKEN_ENV: &str = "DEVE_P2P_INBOUND_TOKEN";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum WsAdmission {
    Browser(AuthSessionId),
    FullPeer,
}

impl WsAdmission {
    #[cfg(test)]
    pub(super) fn is_browser(&self) -> bool {
        matches!(self, Self::Browser(_))
    }

    pub(super) fn browser_auth_session(&self) -> Option<&AuthSessionId> {
        match self {
            Self::Browser(auth_session_id) => Some(auth_session_id),
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

    let token = cookie_token(req);
    if let Some(token) = token.as_deref()
        && jwt::validate_token(&config.secret, token, config.token_version).is_ok()
    {
        return Ok(WsAdmission::Browser(AuthSessionId::from_cookie_token(
            token,
        )));
    }
    if is_browser_session_connection(
        false,
        config.allow_anonymous_localhost,
        is_local_request(req),
    ) {
        return Ok(WsAdmission::Browser(AuthSessionId::anonymous_localhost(
            &config.username,
            config.token_version,
        )));
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
    use axum::http::Request;
    use deve_core::security::auth::config::AuthConfig;
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

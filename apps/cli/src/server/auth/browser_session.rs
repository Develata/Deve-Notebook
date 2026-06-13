//! plan_ref:
//!   - 08_auth#jwt-cookie-contract
//!   - 08_auth#localhost-dev-policy
//!
//! Browser HTTP/WS auth-session resolution.

use crate::server::source_control_grants::AuthSessionId;
use deve_core::protocol::auth::AuthErrorCode;
use deve_core::security::Claims;
use deve_core::security::auth::{config::AuthConfig, jwt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BrowserSessionKind {
    Jwt,
    AnonymousLocalhostDev,
}

#[derive(Debug, Clone)]
pub(crate) struct BrowserSession {
    pub(crate) auth_session_id: AuthSessionId,
    pub(crate) claims: Claims,
    pub(crate) set_cookie: Option<String>,
    kind: BrowserSessionKind,
}

impl BrowserSession {
    pub(crate) fn is_anonymous_localhost_dev(&self) -> bool {
        self.kind == BrowserSessionKind::AnonymousLocalhostDev
    }
}

pub(crate) fn resolve_optional(
    config: &AuthConfig,
    cookie_header: Option<&str>,
    is_loopback: bool,
) -> Option<BrowserSession> {
    if let Some((token, claims)) = valid_jwt_session(config, cookie_header) {
        return Some(BrowserSession {
            auth_session_id: AuthSessionId::from_cookie_token(&token),
            claims,
            set_cookie: None,
            kind: BrowserSessionKind::Jwt,
        });
    }

    if config.allow_anonymous_localhost && is_loopback {
        let dev_session = super::dev_session::resolve_from_cookie_header(
            cookie_header,
            &config.secret,
            super::cookie_policy::secure_cookies_enabled(),
        );
        return Some(BrowserSession {
            auth_session_id: AuthSessionId::from_dev_session_cookie(
                &config.username,
                config.token_version,
                dev_session.value(),
            ),
            claims: anonymous_localhost_claims(config),
            set_cookie: dev_session.set_cookie().map(ToOwned::to_owned),
            kind: BrowserSessionKind::AnonymousLocalhostDev,
        });
    }

    None
}

pub(crate) fn resolve_required(
    config: &AuthConfig,
    cookie_header: Option<&str>,
    is_loopback: bool,
) -> Result<BrowserSession, AuthErrorCode> {
    let token_present = auth_token(cookie_header).is_some();
    resolve_optional(config, cookie_header, is_loopback).ok_or(if token_present {
        AuthErrorCode::TokenExpired
    } else {
        AuthErrorCode::TokenMissing
    })
}

fn valid_jwt_session(config: &AuthConfig, cookie_header: Option<&str>) -> Option<(String, Claims)> {
    let token = auth_token(cookie_header)?;
    let claims = jwt::validate_token(&config.secret, &token, config.token_version).ok()?;
    Some((token, claims))
}

fn auth_token(cookie_header: Option<&str>) -> Option<String> {
    cookie_header.and_then(super::cookie::extract_token_from_cookie_header)
}

fn anonymous_localhost_claims(config: &AuthConfig) -> Claims {
    Claims {
        sub: config.username.clone(),
        iat: 0,
        exp: i64::MAX,
        ver: config.token_version,
        sid: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{BrowserSessionKind, resolve_required};
    use crate::server::auth::dev_session;
    use crate::server::source_control_grants::AuthSessionId;
    use deve_core::security::AuthConfig;
    use deve_core::security::auth::jwt;

    #[test]
    fn anonymous_localhost_auth_prefers_valid_jwt_over_dev_session_cookie() {
        let mut config = AuthConfig::dev_default().expect("auth config");
        config.allow_anonymous_localhost = true;
        let token =
            jwt::issue_token(&config.secret, &config.username, config.token_version).unwrap();
        let dev_cookie = dev_session::cookie_header_for_test(&config.secret, "dev-session");
        let cookie_header = format!("{dev_cookie}; token={token}");

        let session = resolve_required(&config, Some(&cookie_header), true).expect("session");

        assert_eq!(
            session.auth_session_id,
            AuthSessionId::from_cookie_token(&token)
        );
        assert_eq!(session.kind, BrowserSessionKind::Jwt);
        assert!(session.set_cookie.is_none());
    }

    #[test]
    fn jwt_login_tokens_derive_distinct_auth_session_ids() {
        let config = AuthConfig::dev_default().expect("auth config");
        let first =
            jwt::issue_token(&config.secret, &config.username, config.token_version).unwrap();
        let second =
            jwt::issue_token(&config.secret, &config.username, config.token_version).unwrap();

        assert_ne!(
            AuthSessionId::from_cookie_token(&first),
            AuthSessionId::from_cookie_token(&second)
        );
    }

    #[test]
    fn anonymous_localhost_auth_falls_back_to_dev_session_without_valid_jwt() {
        let mut config = AuthConfig::dev_default().expect("auth config");
        config.allow_anonymous_localhost = true;
        let dev_cookie = dev_session::cookie_header_for_test(&config.secret, "dev-session");
        let cookie_header = format!("{dev_cookie}; token=not-a-valid-jwt");

        let session = resolve_required(&config, Some(&cookie_header), true).expect("session");

        assert_eq!(
            session.auth_session_id,
            AuthSessionId::from_dev_session_cookie(
                &config.username,
                config.token_version,
                "dev-session"
            )
        );
        assert_eq!(session.kind, BrowserSessionKind::AnonymousLocalhostDev);
    }
}

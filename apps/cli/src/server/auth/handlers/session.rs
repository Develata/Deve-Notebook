//! plan_ref:
//!   - 09_auth#auth-http-endpoints
//!   - 09_auth#audit
//!   - 09_auth#jwt-cookie-contract

use axum::{
    Extension, Json,
    extract::ConnectInfo,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use std::{net::SocketAddr, sync::Arc};

use deve_core::protocol::auth::{AuthStatusResponse, LoginResponse, MeResponse};
use deve_core::security::AuthConfig;
use deve_core::security::auth::jwt;

const COOKIE_NAME: &str = "token";
const UNKNOWN_USER_AGENT: &str = "unknown";

pub async fn logout() -> impl IntoResponse {
    (
        StatusCode::OK,
        build_removal_cookie(),
        Json(LoginResponse::success()),
    )
}

pub async fn me(Extension(claims): Extension<deve_core::security::Claims>) -> impl IntoResponse {
    Json(MeResponse {
        username: claims.sub,
    })
}

pub async fn status(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(config): Extension<Arc<AuthConfig>>,
) -> impl IntoResponse {
    let authenticated = auth_status_from_cookie_header(
        headers.get("cookie").and_then(|value| value.to_str().ok()),
        addr,
        &config,
    );
    Json(if authenticated {
        AuthStatusResponse::authenticated()
    } else {
        AuthStatusResponse::unauthenticated()
    })
}

pub(super) fn build_auth_cookie(token: &str) -> [(String, String); 1] {
    let cookie = Cookie::build((COOKIE_NAME, token.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .secure(https_enabled())
        .build();
    [("Set-Cookie".into(), cookie.to_string())]
}

fn build_removal_cookie() -> [(String, String); 1] {
    let secure = if https_enabled() { "; Secure" } else { "" };
    let value = format!(
        "{}=; Path=/; HttpOnly; SameSite=Strict{}; Max-Age=0",
        COOKIE_NAME, secure
    );
    [("Set-Cookie".into(), value)]
}

fn https_enabled() -> bool {
    std::env::var("HTTPS_ENABLED")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(true)
}

pub(super) fn build_empty_cookie() -> [(String, String); 1] {
    [("X-No-Op".into(), "1".into())]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LoginAuditEvent {
    success: bool,
    ip: String,
    user: String,
    timestamp: String,
    user_agent: String,
}

pub(super) fn log_login(
    success: bool,
    ip: &std::net::IpAddr,
    user: &str,
    user_agent: Option<&str>,
) {
    let event = login_audit_event(success, ip, user, user_agent);
    if event.success {
        tracing::info!(
            success = event.success,
            user = %event.user,
            ip = %event.ip,
            timestamp = %event.timestamp,
            user_agent = %event.user_agent,
            "Login audit"
        );
    } else {
        tracing::warn!(
            success = event.success,
            user = %event.user,
            ip = %event.ip,
            timestamp = %event.timestamp,
            user_agent = %event.user_agent,
            "Login audit"
        );
    }
}

pub(super) fn login_audit_event(
    success: bool,
    ip: &std::net::IpAddr,
    user: &str,
    user_agent: Option<&str>,
) -> LoginAuditEvent {
    LoginAuditEvent {
        success,
        ip: ip.to_string(),
        user: user.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        user_agent: normalized_user_agent(user_agent),
    }
}

fn normalized_user_agent(user_agent: Option<&str>) -> String {
    user_agent
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(UNKNOWN_USER_AGENT)
        .to_string()
}

fn auth_status_from_cookie_header(
    cookie_header: Option<&str>,
    addr: SocketAddr,
    config: &AuthConfig,
) -> bool {
    if config.allow_anonymous_localhost && addr.ip().is_loopback() {
        return true;
    }
    let Some(token) =
        cookie_header.and_then(super::super::cookie::extract_token_from_cookie_header)
    else {
        return false;
    };
    jwt::validate_token(&config.secret, &token, config.token_version).is_ok()
}

#[cfg(test)]
mod tests {
    use super::{UNKNOWN_USER_AGENT, auth_status_from_cookie_header, login_audit_event};
    use deve_core::security::AuthConfig;
    use deve_core::security::auth::jwt;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn status_is_false_without_token() {
        let config = AuthConfig::dev_default().unwrap();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3001);
        assert!(!auth_status_from_cookie_header(None, addr, &config));
    }

    #[test]
    fn status_accepts_valid_token_cookie() {
        let config = AuthConfig::dev_default().unwrap();
        let token =
            jwt::issue_token(&config.secret, &config.username, config.token_version).unwrap();
        let cookie = format!("token={token}");
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3001);
        assert!(auth_status_from_cookie_header(Some(&cookie), addr, &config));
    }

    #[test]
    fn status_rejects_token_cookie_prefixes() {
        let config = AuthConfig::dev_default().unwrap();
        let token =
            jwt::issue_token(&config.secret, &config.username, config.token_version).unwrap();
        let cookie = format!("token_csrf={token}");
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3001);
        assert!(!auth_status_from_cookie_header(
            Some(&cookie),
            addr,
            &config
        ));
    }

    #[test]
    fn login_audit_event_includes_required_fields() {
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let event = login_audit_event(true, &ip, "admin", Some("DeveTest/1.0"));

        assert!(event.success);
        assert_eq!(event.ip, "127.0.0.1");
        assert_eq!(event.user, "admin");
        assert_eq!(event.user_agent, "DeveTest/1.0");
        assert!(event.timestamp.ends_with('Z'));
    }

    #[test]
    fn login_audit_event_defaults_missing_user_agent() {
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let event = login_audit_event(false, &ip, "admin", Some("   "));

        assert!(!event.success);
        assert_eq!(event.user_agent, UNKNOWN_USER_AGENT);
    }
}

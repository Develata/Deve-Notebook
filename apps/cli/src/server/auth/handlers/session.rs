//! plan_ref:
//!   - 09_auth#auth-http-endpoints
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

pub(super) fn log_login(success: bool, ip: &std::net::IpAddr, user: &str) {
    if success {
        tracing::info!(user = user, ip = %ip, "Login success");
    } else {
        tracing::warn!(user = user, ip = %ip, "Login failed");
    }
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
    use super::auth_status_from_cookie_header;
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
        let token = jwt::issue_token(&config.secret, config.token_version).unwrap();
        let cookie = format!("token={token}");
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3001);
        assert!(auth_status_from_cookie_header(Some(&cookie), addr, &config));
    }

    #[test]
    fn status_rejects_token_cookie_prefixes() {
        let config = AuthConfig::dev_default().unwrap();
        let token = jwt::issue_token(&config.secret, config.token_version).unwrap();
        let cookie = format!("token_csrf={token}");
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3001);
        assert!(!auth_status_from_cookie_header(
            Some(&cookie),
            addr,
            &config
        ));
    }
}

// apps/cli/src/server/auth/middleware.rs
//! plan_ref:
//!   - 08_auth#jwt-cookie-contract
//!   - 08_auth#auth-rate-limiting
//!   - 08_auth#localhost-dev-policy
//!
//! # JWT Cookie 认证中间件
//!
//! 从 HttpOnly Cookie 中提取 JWT，验证后将 Claims 注入请求 Extension。
//!
//! ## Invariants
//! - 未认证请求返回 401 Unauthorized
//! - localhost 免密仅在 `AUTH_ALLOW_ANONYMOUS_LOCALHOST=true` 时生效

use axum::{
    Extension, Json,
    body::Body,
    extract::ConnectInfo,
    http::{HeaderValue, Request, StatusCode, header::SET_COOKIE},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Once};

use crate::server::rate_limit::RateLimiter;
use deve_core::protocol::auth::{AuthErrorCode, AuthErrorResponse, LoginResponse};
use deve_core::security::auth::config::AuthConfig;

static AUTH_BYPASS_WARNING: Once = Once::new();

/// JWT 认证中间件
///
/// 工作流程:
/// 1. 检查 localhost 免密策略
/// 2. 从 Cookie 提取 JWT
/// 3. 验证 JWT 签名 + 有效期 + 版本号
/// 4. 注入 Claims 到 Extension
pub async fn auth_middleware(
    Extension(config): Extension<Arc<AuthConfig>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let cookie_header = req.headers().get("cookie").and_then(|v| v.to_str().ok());
    let session = match super::browser_session::resolve_required(
        &config,
        cookie_header,
        is_localhost(&addr.ip()),
    ) {
        Ok(session) => session,
        Err(code) => return unauthorized(code),
    };

    if session.is_anonymous_localhost_dev() {
        warn_dev_auth_bypass_once();
    }
    req.extensions_mut().insert(session.auth_session_id.clone());
    req.extensions_mut().insert(session.claims.clone());
    let mut response = next.run(req).await;
    if let Some(set_cookie) = session.set_cookie.as_deref()
        && let Ok(value) = HeaderValue::from_str(set_cookie)
    {
        response.headers_mut().append(SET_COOKIE, value);
    }
    response
}

fn warn_dev_auth_bypass_once() {
    AUTH_BYPASS_WARNING.call_once(|| {
        tracing::warn!(
            "WARNING: development-only anonymous localhost auth bypass active; never enable in production"
        );
    });
}

pub async fn login_rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(limiter): Extension<RateLimiter>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if !limiter.check_and_record_ip(addr.ip()) {
        tracing::warn!("Rate limit exceeded for IP: {}", addr.ip());
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [("Retry-After", limiter.retry_after_secs().to_string())],
            Json(LoginResponse::failure(AuthErrorCode::RateLimited)),
        )
            .into_response();
    }
    next.run(req).await
}

pub async fn delegated_source_control_middleware(
    Extension(config): Extension<Arc<AuthConfig>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    match super::delegated_source_control::validate_headers(req.headers(), &config.secret) {
        Ok(()) => next.run(req).await,
        Err(error) => (StatusCode::FORBIDDEN, Json(error)).into_response(),
    }
}

fn is_localhost(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}

fn unauthorized(code: AuthErrorCode) -> Response {
    (StatusCode::UNAUTHORIZED, Json(AuthErrorResponse::new(code))).into_response()
}

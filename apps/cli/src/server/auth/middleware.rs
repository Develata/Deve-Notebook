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
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Once};

use crate::server::rate_limit::RateLimiter;
use deve_core::protocol::auth::{AuthErrorCode, AuthErrorResponse, LoginResponse};
use deve_core::security::auth::{config::AuthConfig, jwt};

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
    // localhost 免密策略
    if config.allow_anonymous_localhost && is_localhost(&addr.ip()) {
        warn_dev_auth_bypass_once();
        let anonymous_claims = deve_core::security::Claims {
            sub: config.username.clone(),
            iat: 0,
            exp: i64::MAX,
            ver: config.token_version,
        };
        req.extensions_mut().insert(anonymous_claims);
        return next.run(req).await;
    }

    // 提取 Cookie
    let token = req
        .headers()
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(super::cookie::extract_token_from_cookie_header);
    let token = match token {
        Some(t) => t,
        None => return unauthorized(AuthErrorCode::TokenMissing),
    };

    // 验证 JWT
    match jwt::validate_token(&config.secret, &token, config.token_version) {
        Ok(claims) => {
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        Err(e) => {
            tracing::debug!("JWT rejected: {:?}", e);
            unauthorized(AuthErrorCode::TokenExpired)
        }
    }
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

fn is_localhost(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}

fn unauthorized(code: AuthErrorCode) -> Response {
    (StatusCode::UNAUTHORIZED, Json(AuthErrorResponse::new(code))).into_response()
}

// apps/cli/src/server/auth/middleware.rs
//! # JWT Cookie 认证中间件
//!
//! 从 HttpOnly Cookie 中提取 JWT，验证后将 Claims 注入请求 Extension。
//!
//! ## Invariants
//! - 未认证请求返回 401 Unauthorized
//! - localhost 免密仅在 `AUTH_ALLOW_ANONYMOUS_LOCALHOST=true` 时生效

use axum::{
    Extension,
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use deve_core::security::auth::{config::AuthConfig, jwt};

const COOKIE_NAME: &str = "token";

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
    let token = extract_cookie_token(&req);
    let token = match token {
        Some(t) => t,
        None => return unauthorized("Missing auth token"),
    };

    // 验证 JWT
    match jwt::validate_token(&config.secret, &token, config.token_version) {
        Ok(claims) => {
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        Err(e) => {
            tracing::debug!("JWT rejected: {:?}", e);
            unauthorized("Invalid or expired token")
        }
    }
}

fn is_localhost(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}

fn extract_cookie_token(req: &Request<Body>) -> Option<String> {
    let header = req.headers().get("cookie")?.to_str().ok()?;
    for pair in header.split(';') {
        let pair = pair.trim();
        if let Some(value) = pair.strip_prefix(COOKIE_NAME) {
            let value = value.trim_start_matches('=');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn unauthorized(msg: &str) -> Response {
    (StatusCode::UNAUTHORIZED, msg.to_string()).into_response()
}

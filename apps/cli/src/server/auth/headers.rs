// apps/cli/src/server/auth/headers.rs
//! # 安全响应头中间件
//!
//! 09_auth.md §安全策略: 所有 HTTP 响应 MUST 包含安全头。
//!
//! ## Headers
//! - `X-Content-Type-Options: nosniff`
//! - `X-Frame-Options: DENY`
//! - `Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval'`

use axum::{
    body::Body,
    http::Request,
    middleware::Next,
    response::Response,
};

/// 安全响应头中间件
pub async fn security_headers(
    req: Request<Body>,
    next: Next,
) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert(
        "X-Content-Type-Options",
        "nosniff".parse().unwrap(),
    );
    headers.insert(
        "X-Frame-Options",
        "DENY".parse().unwrap(),
    );
    headers.insert(
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'"
            .parse()
            .unwrap(),
    );

    response
}

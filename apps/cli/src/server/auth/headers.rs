// apps/cli/src/server/auth/headers.rs
//! plan_ref:
//!   - 09_auth#security-headers
//!
//! # 安全响应头中间件
//!
//! 09_auth#security-headers
//!
//! ## Headers
//! - `X-Content-Type-Options: nosniff`
//! - `X-Frame-Options: DENY`
//! - `Content-Security-Policy`: see `CSP_POLICY`

use axum::{
    body::Body,
    http::{HeaderValue, Request},
    middleware::Next,
    response::Response,
};

const CSP_POLICY: &str = concat!(
    "default-src 'self'; ",
    "script-src 'self' 'wasm-unsafe-eval' 'unsafe-inline'; ",
    "connect-src 'self' ws: wss:; ",
    "img-src 'self' data: blob:; ",
    "style-src 'self' 'unsafe-inline'; ",
    "object-src 'none'; ",
    "frame-ancestors 'none'; ",
    "base-uri 'self'"
);

/// 安全响应头中间件
pub async fn security_headers(req: Request<Body>, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static(CSP_POLICY),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::CSP_POLICY;

    #[test]
    fn csp_allows_current_wasm_bootstrap_without_external_origins() {
        assert!(CSP_POLICY.contains("script-src 'self'"));
        assert!(CSP_POLICY.contains("'wasm-unsafe-eval'"));
        assert!(CSP_POLICY.contains("'unsafe-inline'"));
        assert!(CSP_POLICY.contains("connect-src 'self' ws: wss:"));
        assert!(CSP_POLICY.contains("style-src 'self' 'unsafe-inline'"));
        assert!(!CSP_POLICY.contains("https://cdnjs.cloudflare.com"));
    }
}

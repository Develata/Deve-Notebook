// apps/cli/src/server/auth/handlers.rs
//! # 认证 HTTP 端点
//!
//! | Method | Path               | Auth | Description          |
//! |--------|--------------------|------|----------------------|
//! | POST   | /api/auth/login    | No   | 登录，返回 JWT Cookie |
//! | POST   | /api/auth/logout   | Yes  | 清除 Cookie          |
//! | GET    | /api/auth/me       | Yes  | 返回当前用户信息      |

use axum::{Extension, Json, extract::ConnectInfo, http::StatusCode, response::IntoResponse};
use axum_extra::extract::cookie::{Cookie, SameSite};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;

use deve_core::security::auth::{config::AuthConfig, jwt, password};

use super::brute_force::BruteForceGuard;

const COOKIE_NAME: &str = "token";

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub username: String,
}

/// POST /api/auth/login
///
/// 验证用户名/密码，成功后签发 JWT 写入 HttpOnly Cookie。
pub async fn login(
    Extension(config): Extension<Arc<AuthConfig>>,
    Extension(guard): Extension<Arc<BruteForceGuard>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<LoginRequest>,
) -> impl IntoResponse {
    let ip = addr.ip();

    // 暴力破解检查
    if guard.is_blocked(&ip) {
        tracing::warn!(ip = %ip, "Login blocked (brute force)");
        return (
            StatusCode::TOO_MANY_REQUESTS,
            build_empty_cookie(),
            Json(LoginResponse {
                success: false,
                error: Some("Too many attempts. Try again later.".into()),
            }),
        );
    }

    // 用户名校验
    if body.username != config.username {
        guard.record_failure(&ip);
        log_login(false, &ip, &body.username);
        return (
            StatusCode::UNAUTHORIZED,
            build_empty_cookie(),
            Json(LoginResponse {
                success: false,
                error: Some("Invalid credentials".into()),
            }),
        );
    }

    // 密码校验 (Argon2)
    let ok = password::verify_password(&body.password, &config.password_hash).unwrap_or(false);

    if !ok {
        guard.record_failure(&ip);
        log_login(false, &ip, &body.username);
        return (
            StatusCode::UNAUTHORIZED,
            build_empty_cookie(),
            Json(LoginResponse {
                success: false,
                error: Some("Invalid credentials".into()),
            }),
        );
    }

    // 签发 JWT
    guard.record_success(&ip);
    log_login(true, &ip, &body.username);

    match jwt::issue_token(&config.secret, config.token_version) {
        Ok(token) => {
            let cookie = build_auth_cookie(&token);
            (
                StatusCode::OK,
                cookie,
                Json(LoginResponse {
                    success: true,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!("JWT issue failed: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                build_empty_cookie(),
                Json(LoginResponse {
                    success: false,
                    error: Some("Internal error".into()),
                }),
            )
        }
    }
}

/// POST /api/auth/logout — 清除 Cookie
pub async fn logout() -> impl IntoResponse {
    (
        StatusCode::OK,
        build_removal_cookie(),
        Json(LoginResponse {
            success: true,
            error: None,
        }),
    )
}

/// GET /api/auth/me — 返回已认证用户 (受中间件保护)
pub async fn me(Extension(claims): Extension<deve_core::security::Claims>) -> impl IntoResponse {
    Json(MeResponse {
        username: claims.sub,
    })
}

fn build_auth_cookie(token: &str) -> [(String, String); 1] {
    let cookie = Cookie::build((COOKIE_NAME, token.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .secure(false) // Nginx 终止 TLS 后内部 HTTP
        .build();
    [("Set-Cookie".into(), cookie.to_string())]
}

fn build_removal_cookie() -> [(String, String); 1] {
    // max-age=0 立即过期，清除 Cookie
    let value = format!(
        "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
        COOKIE_NAME
    );
    [("Set-Cookie".into(), value)]
}

fn build_empty_cookie() -> [(String, String); 1] {
    [("X-No-Op".into(), "1".into())]
}

/// 审计日志 (09_auth.md: 登录事件 MUST 记录)
fn log_login(success: bool, ip: &std::net::IpAddr, user: &str) {
    if success {
        tracing::info!(user = user, ip = %ip, "Login success");
    } else {
        tracing::warn!(user = user, ip = %ip, "Login failed");
    }
}

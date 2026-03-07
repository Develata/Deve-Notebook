use axum::{Extension, Json, http::StatusCode, response::IntoResponse};
use axum_extra::extract::cookie::{Cookie, SameSite};

use deve_core::protocol::auth::{LoginResponse, MeResponse};

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

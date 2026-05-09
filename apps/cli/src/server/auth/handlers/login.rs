//! plan_ref:
//!   - 09_auth#auth-http-endpoints
//!   - 09_auth#auth-rate-limiting
//!   - 09_auth#jwt-cookie-contract
//!   - 09_auth#password-hashing

use axum::{
    Extension, Json,
    extract::ConnectInfo,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use std::net::SocketAddr;
use std::sync::Arc;

use deve_core::protocol::auth::{AuthErrorCode, LoginRequest, LoginResponse};
use deve_core::security::auth::{config::AuthConfig, jwt, password};

use crate::server::auth::brute_force::BruteForceGuard;

use super::session::{build_auth_cookie, build_empty_cookie, log_login};

pub async fn login(
    Extension(config): Extension<Arc<AuthConfig>>,
    Extension(guard): Extension<Arc<BruteForceGuard>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<LoginRequest>,
) -> impl IntoResponse {
    let ip = addr.ip();
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok());
    if guard.is_blocked(&ip) {
        tracing::warn!(ip = %ip, "Login blocked (brute force)");
        return reject(StatusCode::TOO_MANY_REQUESTS, AuthErrorCode::RateLimited);
    }

    if body.username != config.username {
        guard.record_failure(&ip);
        log_login(false, &ip, &body.username, user_agent);
        return reject(StatusCode::UNAUTHORIZED, AuthErrorCode::InvalidPassword);
    }

    let ok = match verify_login_password(&body.password, &config.password_hash) {
        Ok(ok) => ok,
        Err(err) => {
            tracing::error!("Password verification failed: {:?}", err);
            return reject(
                StatusCode::INTERNAL_SERVER_ERROR,
                AuthErrorCode::InternalError,
            );
        }
    };
    if !ok {
        guard.record_failure(&ip);
        log_login(false, &ip, &body.username, user_agent);
        return reject(StatusCode::UNAUTHORIZED, AuthErrorCode::InvalidPassword);
    }

    guard.record_success(&ip);
    log_login(true, &ip, &body.username, user_agent);
    match jwt::issue_token(&config.secret, config.token_version) {
        Ok(token) => (
            StatusCode::OK,
            build_auth_cookie(&token),
            Json(LoginResponse::success()),
        ),
        Err(err) => {
            tracing::error!("JWT issue failed: {:?}", err);
            reject(
                StatusCode::INTERNAL_SERVER_ERROR,
                AuthErrorCode::InternalError,
            )
        }
    }
}

fn reject(
    status: StatusCode,
    code: AuthErrorCode,
) -> (StatusCode, [(String, String); 1], Json<LoginResponse>) {
    (
        status,
        build_empty_cookie(),
        Json(LoginResponse::failure(code)),
    )
}

fn verify_login_password(password_input: &str, password_hash: &str) -> anyhow::Result<bool> {
    password::verify_password(password_input, password_hash)
}

#[cfg(test)]
mod tests;
